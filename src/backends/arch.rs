use std::io::Cursor;

use sha2::{Digest, Sha256};
use tar::Archive;

use crate::mirror::MirrorClient;
use crate::mirror_selector::MirrorSelector;
use crate::mirrorlist::MirrorList;
use crate::package::Package;
use crate::root::Root;

pub struct ArchBackend {
    mirrorlist: MirrorList,
    repository: String,
    architecture: String,
}

impl ArchBackend {
    pub fn new(
        mirrorlist: MirrorList,
        repository: impl Into<String>,
        architecture: impl Into<String>,
    ) -> Self {
        Self {
            mirrorlist,
            repository: repository.into(),
            architecture: architecture.into(),
        }
    }

    pub fn install(&self, root: &Root, package: &Package) -> anyhow::Result<()> {
        let filename = package
            .filename
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("package {} has no payload filename", package.name))?;

        let expected_checksum = package
            .checksum
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("package {} has no SHA-256 checksum", package.name))?;

        let mirror = MirrorSelector::select(&self.mirrorlist)?;

        println!("Selected mirror: {}", mirror.name);

        let path = format!("{}/os/{}/{}", self.repository, self.architecture, filename);

        println!("Downloading package: {path}");

        let client = MirrorClient::new()?;
        let data = client.fetch_bytes(mirror, &path)?;

        println!("Downloaded {} bytes", data.len());

        verify_sha256(&data, expected_checksum)?;

        println!("SHA-256 verified.");

        extract_package(root, &data)?;

        Ok(())
    }
}

fn verify_sha256(data: &[u8], expected: &str) -> anyhow::Result<()> {
    let mut hasher = Sha256::new();
    hasher.update(data);

    let digest = hasher.finalize();

    let actual = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();

    if !actual.eq_ignore_ascii_case(expected) {
        anyhow::bail!(
            "package checksum mismatch: expected {}, got {}",
            expected,
            actual
        );
    }

    Ok(())
}

fn extract_package(root: &Root, data: &[u8]) -> anyhow::Result<()> {
    use std::fs;
    use tar::EntryType;

    let decoder = zstd::stream::read::Decoder::new(Cursor::new(data))?;

    let mut archive = Archive::new(decoder);

    println!("Extracting package into {}", root.path().display());

    let mut hard_links = Vec::new();

    for entry in archive.entries()? {
        let mut entry = entry?;

        let path = entry.path()?.into_owned();

        validate_archive_path(&path)?;

        let destination = root.join(&path);

        match entry.header().entry_type() {
            EntryType::Directory => {
                fs::create_dir_all(&destination)?;
            }

            EntryType::Regular => {
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }

                let mut file = fs::File::create(&destination)?;

                std::io::copy(&mut entry, &mut file)?;
            }

            EntryType::Symlink => {
                let target = entry
                    .link_name()?
                    .ok_or_else(|| anyhow::anyhow!("symlink {} has no target", path.display()))?;

                validate_archive_path(&target)?;

                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }

                #[cfg(unix)]
                std::os::unix::fs::symlink(&target, &destination)?;

                #[cfg(not(unix))]
                anyhow::bail!("symlinks are not supported on this platform");
            }

            EntryType::Link => {
                let target = entry
                    .link_name()?
                    .ok_or_else(|| anyhow::anyhow!("hard link {} has no target", path.display()))?;

                validate_archive_path(&target)?;

                hard_links.push((path, target.into_owned()));
            }

            entry_type => {
                println!(
                    "Skipping unsupported archive entry {:?}: {}",
                    entry_type,
                    path.display()
                );
            }
        }
    }

    // Create hard links after all regular files exist.
    for (path, target) in hard_links {
        let destination = root.join(&path);
        let target = root.join(&target);

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        #[cfg(unix)]
        fs::hard_link(&target, &destination)?;

        #[cfg(not(unix))]
        anyhow::bail!("hard links are not supported on this platform");
    }

    Ok(())
}

fn validate_archive_path(path: &std::path::Path) -> anyhow::Result<()> {
    if path.is_absolute() {
        anyhow::bail!(
            "refusing to extract absolute archive path: {}",
            path.display()
        );
    }

    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            anyhow::bail!(
                "refusing to extract unsafe archive path: {}",
                path.display()
            );
        }
    }

    Ok(())
}
