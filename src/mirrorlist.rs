use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MirrorPreference {
    Fastest,
    Order,
}

impl Default for MirrorPreference {
    fn default() -> Self {
        Self::Fastest
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mirror {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorList {
    #[serde(default)]
    pub prefer: MirrorPreference,

    #[serde(default = "default_timeout")]
    pub timeout: u64,

    #[serde(default = "default_retries")]
    pub retries: u32,

    pub mirrors: Vec<Mirror>,
}

fn default_timeout() -> u64 {
    10
}

fn default_retries() -> u32 {
    3
}

impl MirrorList {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let mirrors: Self = toml::from_str(&contents)?;

        mirrors.validate()?;

        Ok(mirrors)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.mirrors.is_empty() {
            anyhow::bail!("mirrorlist contains no mirrors");
        }

        if self.timeout == 0 {
            anyhow::bail!("mirror timeout must be greater than zero");
        }

        for mirror in &self.mirrors {
            if mirror.name.is_empty() {
                anyhow::bail!("mirror name cannot be empty");
            }

            if mirror.url.is_empty() {
                anyhow::bail!("mirror URL cannot be empty");
            }
        }

        Ok(())
    }

    pub fn first(&self) -> &Mirror {
        &self.mirrors[0]
    }
}
