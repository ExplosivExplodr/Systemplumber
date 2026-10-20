use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapConfig {
    pub system: SystemConfig,
    pub kernel: KernelConfig,
    pub init: InitConfig,
    pub bootloader: BootloaderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub hostname: String,
    pub architecture: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfig {
    pub provider: String,
    pub package: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitConfig {
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootloaderConfig {
    pub provider: String,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            system: SystemConfig {
                hostname: "systemplumber".into(),
                architecture: "x86_64".into(),
            },
            kernel: KernelConfig {
                provider: "systemplumber".into(),
                package: "linux".into(),
            },
            init: InitConfig {
                provider: "systemd".into(),
            },
            bootloader: BootloaderConfig {
                provider: "systemd-boot".into(),
            },
        }
    }
}

impl BootstrapConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;

        Ok(())
    }
    pub fn set(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
        match key {
            "system.hostname" => {
                self.system.hostname = value.to_string();
            }

            "system.architecture" => {
                self.system.architecture = value.to_string();
            }

            "kernel.provider" => {
                self.kernel.provider = value.to_string();
            }

            "kernel.package" => {
                self.kernel.package = value.to_string();
            }

            "init.provider" => {
                self.init.provider = value.to_string();
            }

            "bootloader.provider" => {
                self.bootloader.provider = value.to_string();
            }

            _ => {
                anyhow::bail!("unknown bootstrap configuration key: {key}");
            }
        }

        Ok(())
    }
}
