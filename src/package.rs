use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub architecture: String,
    pub description: String,
    pub dependencies: Vec<String>,
}

impl Package {
    pub fn id(&self) -> String {
        format!(
            "{}-{}-{}",
            self.name, self.version, self.architecture
        )
    }
}
