use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub architecture: String,
    pub description: String,
    pub dependencies: Vec<String>,

    #[serde(default)]
    pub repository: Option<String>,

    #[serde(default)]
    pub filename: Option<String>,

    #[serde(default)]
    pub checksum: Option<String>,

    #[serde(default)]
    pub provides: Vec<String>,
}

impl Package {
    pub fn id(&self) -> String {
        format!("{}-{}-{}", self.name, self.version, self.architecture)
    }
}
