#[macro_use]
extern crate log;

mod bot;
mod client;
mod utils;
mod worker;

use serenity::prelude::*;

use std::env;

use bot::Bot;
pub use bot::ImgDb;
pub use worker::Worker;

use client::Client as BuonClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in the environment");
    let guild: u64 = env::var("SERVER_GUILD")
        .map(|s| s.parse().expect("invalid GUILD"))
        .expect("Expected SERVER_GUILD in the environment");

    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let bot = Bot::new(guild).await;

    let mut client = Client::builder(&token, intents)
        .event_handler(bot)
        .await
        .expect("Err creating client");

    client.start().await?;

    Ok(())
}
