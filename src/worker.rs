use std::{
    sync::{atomic::AtomicBool, Arc, RwLock},
    time::{Duration, Instant},
};

use buongiornissimo_rs::Greeting;
use chrono::Local;

use crate::client::Client;

use super::ImgDb;

const GREETINGS_TO_FETCH: &[Greeting] = &[
    Greeting::BuonPomeriggio,
    Greeting::BuonPomeriggio,
    Greeting::Compleanno,
];

/// Base fetch interval
const FETCH_INTERVAL: i64 = 4 * 3600;
const DURATION_ONE_DAY: i64 = 86400;

pub struct Worker {
    db: Arc<RwLock<ImgDb>>,
    next_fetch: Instant,
    should_stop: Arc<AtomicBool>,
}

impl Worker {
    pub fn new(db: Arc<RwLock<ImgDb>>, should_stop: Arc<AtomicBool>) -> Self {
        Self {
            db,
            next_fetch: Instant::now(),
            should_stop,
        }
    }

    pub async fn run(&mut self) {
        loop {
            if self.should_stop() {
                break;
            }

            if self.next_fetch < Instant::now() {
                match self.fetch_all().await {
                    Ok(()) => {
                        self.schedule_next_fetch();
                        info!("images of the day fetched");
                    }
                    Err(e) => {
                        error!("failed to fetch images: {e}");
                    }
                }
            }

            std::thread::sleep(Duration::from_secs(1));
        }
    }

    fn should_stop(&self) -> bool {
        self.should_stop.load(std::sync::atomic::Ordering::Relaxed)
    }

    async fn fetch_all(&mut self) -> anyhow::Result<()> {
        let client = Client::default();
        self.fetch_buongiorno(&client).await?;

        for greeting in GREETINGS_TO_FETCH {
            self.fetch_greeting(&client, *greeting).await?;
        }

        Ok(())
    }

    async fn fetch_buongiorno(&mut self, client: &Client) -> anyhow::Result<()> {
        info!("getting buongiorno...");
        let url = client.get_buongiorno().await?;
        info!("got greeting of the day: {url}");
        let mut db = self
            .db
            .write()
            .map_err(|_| anyhow::anyhow!("failed to write to db"))?;
        db.insert(Greeting::BuonGiorno, url);

        Ok(())
    }

    /// fetch greeting and insert it into db
    async fn fetch_greeting(&mut self, client: &Client, greeting: Greeting) -> anyhow::Result<()> {
        info!("getting {:?}...", greeting);
        let url = client.get_greeting_image(greeting).await?;
        info!("got greeting: {url}");

        let mut db = self
            .db
            .write()
            .map_err(|_| anyhow::anyhow!("failed to write to db"))?;
        db.insert(greeting, url);

        Ok(())
    }

    fn schedule_next_fetch(&mut self) {
        let now = Local::now().timestamp();
        let next_default_interval = now + FETCH_INTERVAL;
        let tomorrow_at_midnight = now - (now % DURATION_ONE_DAY) + DURATION_ONE_DAY;

        let from_now = Duration::from_secs(
            if next_default_interval > tomorrow_at_midnight {
                tomorrow_at_midnight
            } else {
                next_default_interval
            }
            .checked_sub(now)
            .unwrap_or_default() as u64,
        );

        if let Some(next_fetch) = Instant::now().checked_add(from_now) {
            info!(
                "next fetch in {} seconds",
                next_fetch.duration_since(Instant::now()).as_secs()
            );
            self.next_fetch = next_fetch;
        }
    }
}
