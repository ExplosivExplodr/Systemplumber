use crate::database::PackageDatabase;
use crate::mirrorlist::MirrorList;
use crate::repositories::arch::ArchRepository;
use crate::root::Root;

pub struct Sync {
    root: Root,
}

impl Sync {
    pub fn new(root: Root) -> Self {
        Self { root }
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let mirrorlist_path = self.root.mirrorlist();

        let mirrorlist = MirrorList::load(&mirrorlist_path)?;

        println!("Mirror preference: {:?}", mirrorlist.prefer);

        println!("Mirrors available: {}", mirrorlist.mirrors.len());

        let repositories = [
            ArchRepository::new("core", "x86_64"),
            ArchRepository::new("extra", "x86_64"),
            ArchRepository::new("multilib", "x86_64"),
        ];

        let mut all_packages = Vec::new();

        for repository in &repositories {
            println!("\nSyncing repository: {}", repository.name());

            let mut packages = repository.sync(&mirrorlist)?;

            for package in &mut packages {
                package.repository = Some(repository.name().to_string());
            }

            all_packages.extend(packages);
        }

        println!("\nTotal packages synchronized: {}", all_packages.len());

        let database_path = self.root.package_db().join("packages.db");

        let mut database = PackageDatabase::load(&database_path)?;

        database.packages.clear();

        for package in all_packages {
            database.add(package);
        }

        database.save(&database_path)?;

        println!("Package metadata synchronized.");

        println!("Database: {}", database_path.display());

        Ok(())
    }
}
