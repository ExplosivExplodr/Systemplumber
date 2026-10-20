use std::path::Path;

use crate::database::PackageDatabaseReader;
use crate::package::Package;

pub struct PackageCatalog {
    database: PackageDatabaseReader,
}

impl PackageCatalog {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let database = PackageDatabaseReader::open(path)?;

        Ok(Self { database })
    }

    pub fn find(&mut self, name: &str) -> anyhow::Result<Option<Package>> {
        self.database.find(name)
    }

    pub fn find_providers(&mut self, capability: &str) -> anyhow::Result<Vec<Package>> {
        self.database.find_providers(capability)
    }

    pub fn search(&mut self, query: &str) -> anyhow::Result<Vec<Package>> {
        self.database.search(query)
    }
}
