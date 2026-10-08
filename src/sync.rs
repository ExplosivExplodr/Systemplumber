use crate::database::PackageDatabase;
use crate::mirror::MirrorClient;
use crate::mirrorlist::MirrorList;
use crate::repository::Repository;
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

        let mirror = crate::mirror_selector::MirrorSelector::select(&mirrorlist)?;

        println!("Selected mirror: {}", mirror.name);
        println!("Mirror URL: {}", mirror.url);

        let client = MirrorClient::new()?;

        let contents = client.fetch_repository(mirror)?;

        let repository: Repository = toml::from_str(&contents)?;
        repository.validate()?;

        println!("Repository: {}", repository.name);
        println!("Architecture: {}", repository.architecture);
        println!("Packages found: {}", repository.packages.len());

        let database_path = self.root.package_db().join("packages.db");

        let mut database = PackageDatabase::load(&database_path)?;

        database.packages.clear();

        for package in repository.packages {
            database.add(package);
        }

        database.save(&database_path)?;

        println!("Package metadata synchronized.");
        println!("Database: {}", database_path.display());

        Ok(())
    }
}
