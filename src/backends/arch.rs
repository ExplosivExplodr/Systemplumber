use std::io::Cursor;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tar::{Archive, EntryType};

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
        println!(
            "Selected mirror: {}
            ",
            mirror.name
        );

        let path = format!("{}/os/{}/{}", self.repository, self.architecture, filename);
        println!(
            "Downloading package: {path}
            "
        );

        let client = MirrorClient::new()?;
        let data = client.fetch_bytes(mirror, &path)?;

        println!(
            "Downloaded {} bytes
            ",
            data.len()
        );

        verify_sha256(&data, expected_checksum)?;
        println!(
            "SHA-256 verified.
            "
        );

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
            "package checksum mismatch: expected {}, got {}
            ",
            expected,
            actual
        );
    }

    Ok(())
}

fn extract_package(root: &Root, data: &[u8]) -> anyhow::Result<()> {
    use std::fs;

    let decoder = zstd::stream::read::Decoder::new(Cursor::new(data))?;
    let mut archive = Archive::new(decoder);

    println!(
        "Extracting package into {}
            ",
        root.path().display()
    );

    let mut hard_links: Vec<(PathBuf, PathBuf)> = Vec::new();

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

                let mut file = fs::File::create(&destination).map_err(|error| {
                    anyhow::anyhow!(
                        "failed to create {}: {}
                        ",
                        destination.display(),
                        error
                    )
                })?;

                std::io::copy(&mut entry, &mut file)?;
            }

            EntryType::Symlink => {
                let target = entry.link_name()?.ok_or_else(|| {
                    anyhow::anyhow!(
                        "symlink {} has no target
                        ",
                        path.display()
                    )
                })?;

                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }

                match fs::read_link(&destination) {
                    Ok(existing_target) if existing_target == target => {
                        continue;
                    }
                    Ok(existing_target) => {
                        anyhow::bail!(
                            "symlink conflict at {}: existing target is {}, expected {}",
                            destination.display(),
                            existing_target.display(),
                            target.display()
                        );
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(anyhow::anyhow!(
                            "failed to inspect symlink {}: {}
                            ",
                            destination.display(),
                            error
                        ));
                    }
                }

                #[cfg(unix)]
                std::os::unix::fs::symlink(&target, &destination).map_err(|error| {
                    anyhow::anyhow!(
                        "failed to create symlink {} -> {}: {}
                        ",
                        destination.display(),
                        target.display(),
                        error
                    )
                })?;

                #[cfg(not(unix))]
                anyhow::bail!("symlinks are not supported on this platform");
            }

            EntryType::Link => {
                let target = entry
                    .link_name()?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "hard link {} has no target
                        ",
                            path.display()
                        )
                    })?
                    .into_owned();

                validate_archive_path(&target)?;
                hard_links.push((path, target));
            }

            entry_type => {
                println!(
                    "Skipping unsupported archive entry {:?}: {}
                    ",
                    entry_type,
                    path.display()
                );
            }
        }
    }

    // Arch tar hard-link targets are archive-root-relative, not
    // relative to the directory containing the destination.
    //
    // Retry pending links so chains of hard links can be resolved.
    while !hard_links.is_empty() {
        let mut unresolved = Vec::new();
        let mut created_any = false;

        for (path, target) in hard_links {
            let destination = root.join(&path);
            let target_path = root.join(&target);

            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }

            let target_metadata = match fs::metadata(&target_path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    unresolved.push((path, target));
                    continue;
                }
                Err(error) => {
                    return Err(anyhow::anyhow!(
                        "failed to inspect hard-link target {}: {}
                        ",
                        target_path.display(),
                        error
                    ));
                }
            };

            match fs::symlink_metadata(&destination) {
                Ok(_) => {
                    let destination_metadata = fs::metadata(&destination).map_err(|error| {
                        anyhow::anyhow!(
                            "failed to inspect existing hard-link destination {}: {}
                            ",
                            destination.display(),
                            error
                        )
                    })?;

                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::MetadataExt;

                        if destination_metadata.dev() == target_metadata.dev()
                            && destination_metadata.ino() == target_metadata.ino()
                        {
                            created_any = true;
                            continue;
                        }
                    }

                    println!(
                        "DEBUG hardlink conflict: destination={} ({} bytes), target={} ({} bytes)",
                        destination.display(),
                        destination_metadata.len(),
                        target_path.display(),
                        target_metadata.len()
                    );

                    anyhow::bail!(
                        "hard-link destination already exists and differs from target: {} -> {}
                        ",
                        destination.display(),
                        target_path.display()
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(anyhow::anyhow!(
                        "failed to inspect hard-link destination {}: {}
                        ",
                        destination.display(),
                        error
                    ));
                }
            }

            #[cfg(unix)]
            fs::hard_link(&target_path, &destination).map_err(|error| {
                anyhow::anyhow!(
                    "failed to create hard link {} -> {}: {}
                    ",
                    destination.display(),
                    target_path.display(),
                    error
                )
            })?;

            #[cfg(not(unix))]
            anyhow::bail!(
                "hard links are not supported on this platform
                "
            );

            created_any = true;
        }

        if unresolved.is_empty() {
            break;
        }

        if !created_any {
            let (path, target) = &unresolved[0];
            anyhow::bail!(
                "could not resolve hard link {} -> {}: target was not found after retrying pending links",
                root.join(path).display(),
                root.join(target).display()
            );
        }

        hard_links = unresolved;
    }

    Ok(())
}

fn validate_archive_path(path: &Path) -> anyhow::Result<()> {
    if path.is_absolute() {
        anyhow::bail!(
            "refusing to extract absolute archive path: {}
            ",
            path.display()
        );
    }

    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            anyhow::bail!(
                "refusing to extract unsafe archive path: {}
                ",
                path.display()
            );
        }
    }

    Ok(())
}
