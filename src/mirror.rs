use std::fs;
use std::path::Path;

use reqwest::blocking::Client;
use url::Url;

use crate::mirrorlist::Mirror;

pub struct MirrorClient {
    client: Client,
}

impl MirrorClient {
    pub fn new() -> anyhow::Result<Self> {
        let client = Client::builder().build()?;

        Ok(Self { client })
    }

    pub fn fetch_repository(&self, mirror: &Mirror) -> anyhow::Result<String> {
        if mirror.url.starts_with("http://") || mirror.url.starts_with("https://") {
            self.fetch_http(mirror)
        } else {
            self.fetch_local(mirror)
        }
    }

    fn fetch_http(&self, mirror: &Mirror) -> anyhow::Result<String> {
        let base = Url::parse(&mirror.url)?;
        let url = base.join("repository.toml")?;

        let response = self.client.get(url.clone()).send()?;

        if !response.status().is_success() {
            anyhow::bail!(
                "mirror {} returned HTTP status {}",
                mirror.name,
                response.status()
            );
        }

        Ok(response.text()?)
    }

    fn fetch_local(&self, mirror: &Mirror) -> anyhow::Result<String> {
        let path = Path::new(&mirror.url).join("repository.toml");

        Ok(fs::read_to_string(path)?)
    }
}
