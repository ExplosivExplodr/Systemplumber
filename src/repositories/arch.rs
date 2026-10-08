use crate::mirror::MirrorClient;
use crate::mirror_selector::MirrorSelector;
use crate::mirrorlist::MirrorList;
use crate::package::Package;
use crate::parsers::arch;

pub struct ArchRepository {
    name: String,
    architecture: String,
}

impl ArchRepository {
    pub fn new(name: impl Into<String>, architecture: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            architecture: architecture.into(),
        }
    }

    pub fn database_path(&self) -> String {
        format!("{}/os/{}/{}.db", self.name, self.architecture, self.name)
    }

    pub fn sync(&self, mirrorlist: &MirrorList) -> anyhow::Result<Vec<Package>> {
        let mirror = MirrorSelector::select(mirrorlist)?;

        println!("Selected mirror: {}", mirror.name);
        println!("Mirror URL: {}", mirror.url);

        let client = MirrorClient::new()?;

        let path = self.database_path();

        println!("Fetching Arch repository database: {path}");

        let data = client.fetch_bytes(mirror, &path)?;

        println!("Downloaded {} bytes", data.len());

        let packages = arch::parse_repository(&data[..])?;

        println!("Parsed {} packages", packages.len());

        Ok(packages)
    }
}
