use buongiornissimo_rs::{BuongiornissimoCaffe, Greeting, IlMondoDiGrazia, Scrape, ScrapeError};
use chrono::Local;
use url::Url;

use crate::utils::random as random_utils;

#[derive(Default)]
pub struct Client;

impl Client {
    pub async fn get_buongiorno(&self) -> anyhow::Result<Url> {
        self.get_greeting_image(buongiornissimo_rs::greeting_of_the_day(
            Local::now().date_naive(),
            *random_utils::choice(&[true, false]),
        ))
        .await
    }

    /// Get greeting image for media type.
    /// At the first try it'll use a random provider; then if the media type is not supported, it tries all the different providers
    pub async fn get_greeting_image(&self, media: Greeting) -> anyhow::Result<Url> {
        match random_utils::choice(&[0, 1]) {
            0 => Self::do_get_greeting_image(IlMondoDiGrazia::default(), media).await,
            _ => Self::do_get_greeting_image(BuongiornissimoCaffe::default(), media).await,
        }
    }

    async fn do_get_greeting_image(
        provider: impl Scrape,
        greeting: Greeting,
    ) -> anyhow::Result<Url> {
        match provider.scrape(greeting).await {
            Ok(urls) => Ok(random_utils::choice(&urls).clone()),
            Err(ScrapeError::UnsupportedGreeting) => Self::try_all_providers(greeting).await,
            Err(err) => anyhow::bail!("failed to scrape image: {}", err),
        }
    }

    async fn try_all_providers(media: Greeting) -> anyhow::Result<Url> {
        if let Ok(urls) = IlMondoDiGrazia::default().scrape(media).await {
            return Ok(random_utils::choice(&urls).clone());
        }
        if let Ok(urls) = BuongiornissimoCaffe::default().scrape(media).await {
            return Ok(random_utils::choice(&urls).clone());
        }

        // try to get buongiorno
        match random_utils::choice(&[0, 1]) {
            0 => IlMondoDiGrazia::default()
                .scrape(Greeting::BuonGiorno)
                .await
                .map(|urls| random_utils::choice(&urls).clone())
                .map_err(|e| anyhow::anyhow!("failed to scrape image: {}", e)),
            _ => BuongiornissimoCaffe::default()
                .scrape(Greeting::BuonGiorno)
                .await
                .map(|urls| random_utils::choice(&urls).clone())
                .map_err(|e| anyhow::anyhow!("failed to scrape image: {}", e)),
        }
    }
}
