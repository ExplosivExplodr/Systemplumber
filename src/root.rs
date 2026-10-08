use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Root {
    path: PathBuf,
}

impl Root {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
    pub fn systemplumber_config(&self) -> PathBuf {
        self.etc().join("systemplumber")
    }

    pub fn bootstrap_config(&self) -> PathBuf {
        self.systemplumber_config().join("bootstrap.toml")
    }
    pub fn host() -> Self {
        Self::new("/")
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn join(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();

        if path == Path::new("/") {
            return self.path.clone();
        }

        self.path.join(path.strip_prefix("/").unwrap_or(path))
    }

    pub fn etc(&self) -> PathBuf {
        self.join("/etc")
    }

    pub fn usr(&self) -> PathBuf {
        self.join("/usr")
    }

    pub fn var(&self) -> PathBuf {
        self.join("/var")
    }

    pub fn boot(&self) -> PathBuf {
        self.join("/boot")
    }

    pub fn package_db(&self) -> PathBuf {
        self.var().join("lib/systemplumber")
    }
    pub fn mirrorlist(&self) -> PathBuf {
        self.systemplumber_config().join("mirrorlist.toml")
    }
}
