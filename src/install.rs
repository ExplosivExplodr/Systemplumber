use crate::bootstrap_init::BootstrapSystem;
use crate::package::Package;

pub struct Installer {
    system: BootstrapSystem,
}

impl Installer {
    pub fn new(system: BootstrapSystem) -> Self {
        Self { system }
    }

    pub fn prepare(&self) -> anyhow::Result<()> {
        self.system.initialize()
    }

    pub fn install(&self, package: &Package) -> anyhow::Result<()> {
        self.prepare()?;

        println!("Installing {} {}", package.name, package.version);

        Ok(())
    }
}
