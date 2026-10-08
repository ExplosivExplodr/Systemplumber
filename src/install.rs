use crate::backends::arch::ArchBackend;
use crate::bootstrap_init::BootstrapSystem;
use crate::catalog::PackageCatalog;
use crate::mirrorlist::MirrorList;
use crate::package::Package;

pub struct Installer {
    system: BootstrapSystem,
    catalog: PackageCatalog,
    mirrorlist: MirrorList,
}

impl Installer {
    pub fn new(system: BootstrapSystem, catalog: PackageCatalog, mirrorlist: MirrorList) -> Self {
        Self {
            system,
            catalog,
            mirrorlist,
        }
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

        let repository = package.repository.as_deref().ok_or_else(|| {
            anyhow::anyhow!("package {} has no repository provenance", package.name)
        })?;

        let backend = ArchBackend::new(self.mirrorlist.clone(), repository, &package.architecture);

        backend.install(&self.system.root, package)?;

        println!("Installed {} {}", package.name, package.version);

        Ok(())
    }
}
