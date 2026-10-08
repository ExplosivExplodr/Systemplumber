use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::package::Package;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub architecture: String,

    #[serde(default)]
    pub packages: Vec<Package>,
}

impl Repository {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let repository = toml::from_str(&contents)?;

        Ok(repository)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.name.is_empty() {
            anyhow::bail!("repository name cannot be empty");
        }

        if self.architecture.is_empty() {
            anyhow::bail!("repository architecture cannot be empty");
        }

        Ok(())
    }
}
