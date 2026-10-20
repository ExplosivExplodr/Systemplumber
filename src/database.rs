use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::package::Package;

const MAGIC: &[u8; 4] = b"SPDB";
const VERSION: u32 = 5;
const HEADER_SIZE: u64 = 32;

#[derive(Debug, Default)]
pub struct PackageDatabase {
    pub packages: Vec<Package>,
}

#[derive(Debug)]
struct IndexEntry {
    name: String,
    offset: u64,
    length: u64,
}

pub struct PackageDatabaseReader {
    file: File,
    index: Vec<IndexEntry>,
}

impl PackageDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let mut file = File::open(path)?;

        read_header(&mut file)?;

        let package_count = read_u64(&mut file)?;
        let index_offset = read_u64(&mut file)?;
        let records_offset = read_u64(&mut file)?;

        validate_offsets(index_offset, records_offset)?;

        file.seek(SeekFrom::Start(index_offset))?;

        let mut index = Vec::with_capacity(package_count as usize);

        for _ in 0..package_count {
            let name = read_string(&mut file)?;
            let offset = read_u64(&mut file)?;
            let length = read_u64(&mut file)?;

            index.push(IndexEntry {
                name,
                offset,
                length,
            });
        }

        let mut packages = Vec::with_capacity(package_count as usize);

        for entry in index {
            file.seek(SeekFrom::Start(entry.offset))?;

            let package = read_package(&mut file)?;

            let consumed = file.stream_position()? - entry.offset;

            if consumed != entry.length {
                anyhow::bail!("package record length mismatch for {}", entry.name);
            }

            packages.push(package);
        }

        Ok(Self { packages })
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut packages = self.packages.clone();

        // The index must be sorted because PackageDatabaseReader
        // uses binary search.
        packages.sort_by(|a, b| a.name.cmp(&b.name));

        let records_offset = HEADER_SIZE;

        let mut records = Vec::new();
        let mut index = Vec::with_capacity(packages.len());

        for package in &packages {
            let offset = records_offset + records.len() as u64;

            let mut record = Vec::new();
            write_package(&mut record, package)?;

            let length = record.len() as u64;

            records.extend_from_slice(&record);

            index.push(IndexEntry {
                name: package.name.clone(),
                offset,
                length,
            });
        }

        let index_offset = records_offset + records.len() as u64;

        let mut file = File::create(path)?;

        // Header
        file.write_all(MAGIC)?;
        file.write_all(&VERSION.to_le_bytes())?;
        file.write_all(&(packages.len() as u64).to_le_bytes())?;
        file.write_all(&index_offset.to_le_bytes())?;
        file.write_all(&records_offset.to_le_bytes())?;

        // Package records
        file.write_all(&records)?;

        // Index
        for entry in &index {
            write_string(&mut file, &entry.name)?;
            file.write_all(&entry.offset.to_le_bytes())?;
            file.write_all(&entry.length.to_le_bytes())?;
        }

        file.flush()?;

        Ok(())
    }

    pub fn add(&mut self, package: Package) {
        self.packages.push(package);
    }

    pub fn find(&self, name: &str) -> Option<&Package> {
        self.packages.iter().find(|package| package.name == name)
    }
}

impl PackageDatabaseReader {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let mut file = File::open(path)?;

        read_header(&mut file)?;

        let package_count = read_u64(&mut file)?;
        let index_offset = read_u64(&mut file)?;
        let records_offset = read_u64(&mut file)?;

        validate_offsets(index_offset, records_offset)?;

        file.seek(SeekFrom::Start(index_offset))?;

        let mut index = Vec::with_capacity(package_count as usize);

        for _ in 0..package_count {
            let name = read_string(&mut file)?;
            let offset = read_u64(&mut file)?;
            let length = read_u64(&mut file)?;

            index.push(IndexEntry {
                name,
                offset,
                length,
            });
        }

        Ok(Self { file, index })
    }

    pub fn find(&mut self, name: &str) -> anyhow::Result<Option<Package>> {
        let entry = match self
            .index
            .binary_search_by(|entry| entry.name.as_str().cmp(name))
        {
            Ok(index) => &self.index[index],
            Err(_) => return Ok(None),
        };

        self.file.seek(SeekFrom::Start(entry.offset))?;

        let package = read_package(&mut self.file)?;

        let consumed = self.file.stream_position()? - entry.offset;

        if consumed != entry.length {
            anyhow::bail!("package record length mismatch for {}", entry.name);
        }

        Ok(Some(package))
    }

    pub fn find_providers(&mut self, capability: &str) -> anyhow::Result<Vec<Package>> {
        let mut providers = Vec::new();

        for entry in &self.index {
            self.file.seek(SeekFrom::Start(entry.offset))?;

            let package = read_package(&mut self.file)?;

            let consumed = self.file.stream_position()? - entry.offset;

            if consumed != entry.length {
                anyhow::bail!("package record length mismatch for {}", entry.name);
            }

            let provides_capability = package.provides.iter().any(|provide| {
                let name = provide
                    .split_once('=')
                    .map(|(name, _)| name)
                    .unwrap_or(provide.as_str());

                name == capability
            });

            if provides_capability {
                providers.push(package);
            }
        }

        Ok(providers)
    }

    pub fn search(&mut self, query: &str) -> anyhow::Result<Vec<Package>> {
        let query = query.to_lowercase();

        let mut results = Vec::new();

        for entry in &self.index {
            if entry.name.to_lowercase().contains(&query) {
                self.file.seek(SeekFrom::Start(entry.offset))?;

                let package = read_package(&mut self.file)?;

                let consumed = self.file.stream_position()? - entry.offset;

                if consumed != entry.length {
                    anyhow::bail!("package record length mismatch for {}", entry.name);
                }

                results.push(package);
            }
        }

        Ok(results)
    }
}

fn validate_offsets(index_offset: u64, records_offset: u64) -> anyhow::Result<()> {
    if index_offset < HEADER_SIZE {
        anyhow::bail!("invalid package database index offset");
    }

    if records_offset < HEADER_SIZE {
        anyhow::bail!("invalid package database records offset");
    }

    if index_offset < records_offset {
        anyhow::bail!("invalid package database records offset");
    }

    Ok(())
}

fn read_header(file: &mut File) -> anyhow::Result<()> {
    let mut magic = [0u8; 4];

    file.read_exact(&mut magic)?;

    if &magic != MAGIC {
        anyhow::bail!("invalid package database magic");
    }

    let version = read_u32(file)?;

    if version != VERSION {
        anyhow::bail!("unsupported package database version: {version}");
    }

    Ok(())
}

fn write_package(file: &mut impl Write, package: &Package) -> anyhow::Result<()> {
    write_string(file, &package.name)?;
    write_string(file, &package.version)?;
    write_string(file, &package.architecture)?;
    write_string(file, &package.description)?;

    let dependency_count = u32::try_from(package.dependencies.len())
        .map_err(|_| anyhow::anyhow!("too many dependencies"))?;

    file.write_all(&dependency_count.to_le_bytes())?;

    for dependency in &package.dependencies {
        write_string(file, dependency)?;
    }

    let provides_count =
        u32::try_from(package.provides.len()).map_err(|_| anyhow::anyhow!("too many provides"))?;

    file.write_all(&provides_count.to_le_bytes())?;

    for provide in &package.provides {
        write_string(file, provide)?;
    }

    write_optional_string(file, package.repository.as_deref())?;
    write_optional_string(file, package.filename.as_deref())?;
    write_optional_string(file, package.checksum.as_deref())?;

    Ok(())
}

fn read_package(file: &mut File) -> anyhow::Result<Package> {
    let name = read_string(file)?;
    let version = read_string(file)?;
    let architecture = read_string(file)?;
    let description = read_string(file)?;

    let dependency_count = read_u32(file)?;

    let mut dependencies = Vec::with_capacity(dependency_count as usize);

    for _ in 0..dependency_count {
        dependencies.push(read_string(file)?);
    }

    let provides_count = read_u32(file)?;

    let mut provides = Vec::with_capacity(provides_count as usize);

    for _ in 0..provides_count {
        provides.push(read_string(file)?);
    }

    let repository = read_optional_string(file)?;
    let filename = read_optional_string(file)?;
    let checksum = read_optional_string(file)?;

    Ok(Package {
        name,
        version,
        architecture,
        description,
        dependencies,
        provides,
        repository,
        filename,
        checksum,
    })
}

fn write_string(file: &mut impl Write, value: &str) -> anyhow::Result<()> {
    let bytes = value.as_bytes();

    let length = u32::try_from(bytes.len()).map_err(|_| anyhow::anyhow!("string is too long"))?;

    file.write_all(&length.to_le_bytes())?;
    file.write_all(bytes)?;

    Ok(())
}

fn read_string(file: &mut File) -> anyhow::Result<String> {
    let length = read_u32(file)?;

    let mut bytes = vec![0u8; length as usize];

    file.read_exact(&mut bytes)?;

    Ok(String::from_utf8(bytes)?)
}

fn read_u32(file: &mut File) -> anyhow::Result<u32> {
    let mut bytes = [0u8; 4];

    file.read_exact(&mut bytes)?;

    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(file: &mut File) -> anyhow::Result<u64> {
    let mut bytes = [0u8; 8];

    file.read_exact(&mut bytes)?;

    Ok(u64::from_le_bytes(bytes))
}
fn write_optional_string(file: &mut impl Write, value: Option<&str>) -> anyhow::Result<()> {
    match value {
        Some(value) => {
            file.write_all(&[1])?;
            write_string(file, value)?;
        }

        None => {
            file.write_all(&[0])?;
        }
    }

    Ok(())
}

fn read_optional_string(file: &mut File) -> anyhow::Result<Option<String>> {
    let mut flag = [0u8; 1];

    file.read_exact(&mut flag)?;

    match flag[0] {
        0 => Ok(None),
        1 => Ok(Some(read_string(file)?)),
        _ => anyhow::bail!("invalid optional string flag"),
    }
}
