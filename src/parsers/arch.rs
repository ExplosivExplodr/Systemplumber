use std::io::{BufRead, BufReader, Read};

use tar::Archive;

use crate::package::Package;

pub fn parse_desc(reader: impl Read) -> anyhow::Result<Package> {
    let reader = BufReader::new(reader);

    let mut name = None;
    let mut version = None;
    let mut architecture = None;
    let mut description = None;
    let mut dependencies = Vec::new();
    let mut filename = None;
    let mut provides = Vec::new();
    let mut checksum = None;
    let mut section = String::new();

    for line in reader.lines() {
        let line = line?;

        if line.is_empty() {
            continue;
        }

        if line.starts_with('%') && line.ends_with('%') {
            section = line;
            continue;
        }

        match section.as_str() {
            "%NAME%" => name = Some(line),
            "%VERSION%" => version = Some(line),
            "%ARCH%" => architecture = Some(line),
            "%DESC%" => description = Some(line),
            "%PROVIDES%" => provides.push(line),

            "%FILENAME%" => {
                filename = Some(line);
            }

            "%SHA256SUM%" => {
                checksum = Some(line);
            }

            "%DEPENDS%" => {
                dependencies.push(line);
            }

            _ => {}
        }
    }

    Ok(Package {
        name: name.ok_or_else(|| anyhow::anyhow!("missing %NAME%"))?,
        version: version.ok_or_else(|| anyhow::anyhow!("missing %VERSION%"))?,
        architecture: architecture.ok_or_else(|| anyhow::anyhow!("missing %ARCH%"))?,
        description: description.unwrap_or_default(),
        dependencies,
        repository: None,
        filename,
        provides,
        checksum,
    })
}

fn parse_tar<R: Read>(reader: R) -> anyhow::Result<Vec<Package>> {
    let mut archive = Archive::new(reader);

    let mut packages = Vec::new();

    for entry in archive.entries()? {
        let mut entry = entry?;

        let path = entry.path()?;

        if path.file_name().and_then(|name| name.to_str()) != Some("desc") {
            continue;
        }

        let package = parse_desc(&mut entry)?;

        packages.push(package);
    }

    Ok(packages)
}

pub fn parse_repository<R: Read>(mut reader: R) -> anyhow::Result<Vec<Package>> {
    use flate2::read::GzDecoder;

    let mut data = Vec::new();
    reader.read_to_end(&mut data)?;

    // Zstandard
    if data.starts_with(&[0x28, 0xB5, 0x2F, 0xFD]) {
        let decoder = zstd::stream::read::Decoder::new(&data[..])?;
        return parse_tar(decoder);
    }

    // Gzip
    if data.starts_with(&[0x1F, 0x8B]) {
        let decoder = GzDecoder::new(&data[..]);
        return parse_tar(decoder);
    }

    // Uncompressed tar
    parse_tar(&data[..])
}
