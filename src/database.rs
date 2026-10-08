use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::package::Package;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PackageDatabase {
    pub packages: Vec<Package>,
}

impl PackageDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let contents = fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        fs::write(path, contents)?;

        Ok(())
    }

    pub fn add(&mut self, package: Package) {
        self.packages.push(package);
    }

    pub fn find(&self, name: &str) -> Option<&Package> {
        self.packages.iter().find(|package| package.name == name)
    }
}
