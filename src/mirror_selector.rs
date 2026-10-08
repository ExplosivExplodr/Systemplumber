use std::time::{Duration, Instant};

use crate::mirrorlist::{Mirror, MirrorList, MirrorPreference};

pub struct MirrorSelector;

impl MirrorSelector {
    pub fn select(mirrorlist: &MirrorList) -> anyhow::Result<&Mirror> {
        match mirrorlist.prefer {
            MirrorPreference::Order => Ok(mirrorlist.first()),
            MirrorPreference::Fastest => Self::select_fastest(mirrorlist),
        }
    }

    fn select_fastest(mirrorlist: &MirrorList) -> anyhow::Result<&Mirror> {
        let mut fastest: Option<(&Mirror, Duration)> = None;

        for mirror in &mirrorlist.mirrors {
            let start = Instant::now();

            if Self::probe(mirror).is_err() {
                continue;
            }

            let elapsed = start.elapsed();

            match fastest {
                Some((_, best_time)) if elapsed >= best_time => {}
                _ => {
                    fastest = Some((mirror, elapsed));
                }
            }
        }

        fastest
            .map(|(mirror, _)| mirror)
            .ok_or_else(|| anyhow::anyhow!("no available mirrors"))
    }

    fn probe(mirror: &Mirror) -> anyhow::Result<()> {
        if mirror.url.starts_with("http://") || mirror.url.starts_with("https://") {
            let client = reqwest::blocking::Client::new();

            let response = client
                .head(&mirror.url)
                .timeout(Duration::from_secs(5))
                .send()?;

            if !response.status().is_success() {
                anyhow::bail!("mirror returned HTTP status {}", response.status());
            }

            return Ok(());
        }

        std::fs::metadata(&mirror.url)?;

        Ok(())
    }
}
