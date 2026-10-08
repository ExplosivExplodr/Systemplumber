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
        let bytes = self.fetch_bytes(mirror, "repository.toml")?;

        Ok(String::from_utf8(bytes)?)
    }

    pub fn fetch_bytes(&self, mirror: &Mirror, path: &str) -> anyhow::Result<Vec<u8>> {
        if mirror.url.starts_with("http://") || mirror.url.starts_with("https://") {
            self.fetch_http(mirror, path)
        } else {
            self.fetch_local(mirror, path)
        }
    }

    fn fetch_http(&self, mirror: &Mirror, path: &str) -> anyhow::Result<Vec<u8>> {
        let mut base = Url::parse(&mirror.url)?;

        if !base.path().ends_with('/') {
            base.path_segments_mut()
                .map_err(|_| anyhow::anyhow!("invalid mirror URL"))?
                .pop_if_empty();
        }

        let url = base.join(path)?;

        let response = self.client.get(url.clone()).send()?;

        if !response.status().is_success() {
            anyhow::bail!(
                "mirror {} returned HTTP status {}",
                mirror.name,
                response.status()
            );
        }

        Ok(response.bytes()?.to_vec())
    }

    fn fetch_local(&self, mirror: &Mirror, path: &str) -> anyhow::Result<Vec<u8>> {
        let path = Path::new(&mirror.url).join(path);

        Ok(fs::read(path)?)
    }
}
