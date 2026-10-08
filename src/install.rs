use crate::bootstrap_init::BootstrapSystem;
use crate::catalog::PackageCatalog;
use crate::package::Package;

pub struct Installer {
    system: BootstrapSystem,
    catalog: PackageCatalog,
}

impl Installer {
    pub fn new(system: BootstrapSystem, catalog: PackageCatalog) -> Self {
        Self { system, catalog }
    }

    pub fn prepare(&self) -> anyhow::Result<()> {
        self.system.initialize()
    }

    pub fn install(&mut self, name: &str) -> anyhow::Result<()> {
        self.prepare()?;

        let package = self
            .catalog
            .find(name)?
            .ok_or_else(|| anyhow::anyhow!("package not found: {name}"))?;

        self.install_package(&package)
    }

    fn install_package(&self, package: &Package) -> anyhow::Result<()> {
        println!("Installing {} {}", package.name, package.version);

        Ok(())
    }
}
