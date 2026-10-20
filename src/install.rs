use crate::backends::arch::ArchBackend;
use crate::bootstrap_init::BootstrapSystem;
use crate::catalog::PackageCatalog;
use crate::dependency::DependencyResolver;
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

        let mut resolver = DependencyResolver::new(&mut self.catalog);

        let plan = resolver.resolve(&[name.to_string()])?;

        println!("Resolved {} package(s):", plan.packages.len());

        for package in &plan.packages {
            println!("  {} {}", package.name, package.version);
        }

        for package in &plan.packages {
            if let Err(error) = self.install_package(package) {
                anyhow::bail!("failed to install {}: {}", package.name, error);
            }
        }

        Ok(())
    }

    fn install_package(&self, package: &Package) -> anyhow::Result<()> {
        println!("Installing {} {}", package.name, package.version);

        let repository = package.repository.as_deref().ok_or_else(|| {
            anyhow::anyhow!("package {} has no repository provenance", package.name)
        })?;

        let backend = ArchBackend::new(self.mirrorlist.clone(), repository, "x86_64");

        backend.install(&self.system.root, package)?;

        println!("Installed {} {}", package.name, package.version);

        Ok(())
    }
}
