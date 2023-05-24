mod response;

use super::BuonClient;
use super::Worker;
use response::{Response, ResponseTokens};

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};

use buongiornissimo_rs::Greeting;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::application_command::{
    CommandDataOption, CommandDataOptionValue,
};
use serenity::model::prelude::AttachmentType;
use serenity::prelude::*;
use serenity::{async_trait, model::prelude::GuildId};
use tokio::task::JoinHandle;
use tracing::info;
use url::Url;

const CMD_BUONGIORNISSIMO: &str = "buongiornissimo";
const CMD_AUGURI: &str = "auguri";
const CMD_BUONPOMERIGGIO: &str = "buonpomeriggio";
const CMD_BUONANOTTE: &str = "buonanotte";

pub type ImgDb = HashMap<Greeting, Url>;

#[allow(dead_code)]
pub struct Bot {
    client: BuonClient,
    db: Arc<RwLock<ImgDb>>,
    guild: u64,
    worker_join_handle: JoinHandle<()>,
    worker_should_stop: Arc<AtomicBool>,
}

impl Bot {
    pub async fn new(guild: u64) -> Self {
        let db = Arc::new(RwLock::new(ImgDb::default()));
        let db_worker: Arc<RwLock<HashMap<Greeting, Url>>> = db.clone();
        let worker_should_stop = Arc::new(AtomicBool::new(false));
        let worker_should_stop_int = worker_should_stop.clone();
        // spawn worker
        info!("starting worker...");
        let worker_join_handle =
            tokio::spawn(async move { Worker::new(db_worker, worker_should_stop_int).run().await });
        info!("worker started!");

        Self {
            client: BuonClient::default(),
            db,
            guild,
            worker_join_handle,
            worker_should_stop,
        }
    }
}

impl Drop for Bot {
    fn drop(&mut self) {
        info!("stopping worker");
        self.worker_should_stop
            .store(true, std::sync::atomic::Ordering::Relaxed);

        info!("worker stopped!");
    }
}

#[async_trait]
impl EventHandler for Bot {
    async fn message(&self, _ctx: Context, _msg: Message) {}

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        let guild_id = GuildId(self.guild);

        GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            commands
                .create_application_command(|command| {
                    command
                        .name(CMD_BUONGIORNISSIMO)
                        .description("Ottieni un'immagine del buongiorno")
                })
                .create_application_command(|command| {
                    command
                        .name(CMD_AUGURI)
                        .description("Ottieni un'immagine per fare gli auguri di buon compleanno")
                        .create_option(|option| {
                            option
                                .name("user")
                                .description("the user to whose birthday is today")
                                .kind(CommandOptionType::User)
                                .required(true)
                        })
                })
                .create_application_command(|command| {
                    command
                        .name(CMD_BUONANOTTE)
                        .description("Ottieni un'immagine delle buona notte")
                })
                .create_application_command(|command| {
                    command
                        .name(CMD_BUONPOMERIGGIO)
                        .description("Ottieni un'immagine del buon pomeriggio")
                })
        })
        .await
        .unwrap();
    }

    // `interaction_create` runs when the user interacts with the bot
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        // check if the interaction is a command
        if let Interaction::ApplicationCommand(command) = interaction {
            let response_content = match command.data.name.as_str() {
                CMD_BUONGIORNISSIMO => self.get_image(Greeting::BuonGiorno),
                CMD_AUGURI => self.happy_birthday(&command.data.options),
                CMD_BUONANOTTE => self.get_image(Greeting::BuonaNotte),
                CMD_BUONPOMERIGGIO => self.get_image(Greeting::BuonPomeriggio),
                command => unreachable!("Comando sconosciuto: {}", command),
            };

            let response_content = match response_content {
                Ok(r) => r,
                Err(e) => {
                    error!("failed to get response {e}");
                    unreachable!("Impossibile elaborare la risposta");
                }
            };

            // send `response_content` to the discord server
            command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            for token in response_content.tokens {
                                match token {
                                    ResponseTokens::File(file) => {
                                        message.add_file(AttachmentType::Image(file))
                                    }
                                    ResponseTokens::Text(text) => message.content(text),
                                };
                            }

                            message
                        })
                })
                .await
                .expect("Cannot respond to slash command");
        }
    }
}

impl Bot {
    fn get_image(&self, greeting: Greeting) -> anyhow::Result<Response> {
        let db = self
            .db
            .read()
            .map_err(|_| anyhow::anyhow!("could not read from db"))?;

        match db.get(&greeting) {
            Some(url) => Ok(Response::default().file(url.clone())),
            None => {
                error!(
                    "could not find any image for {:?} (is the worker still fetching?)",
                    greeting
                );
                Ok(Response::default().text(
                    "Riprova tra un attimo, che sto cercando ancora le miglior immagini"
                        .to_string(),
                ))
            }
        }
    }

    fn happy_birthday(&self, options: &[CommandDataOption]) -> anyhow::Result<Response> {
        let user = options.get(0).unwrap().resolved.as_ref().unwrap();
        if let CommandDataOptionValue::User(user, _) = user {
            Ok(self
                .get_image(Greeting::Compleanno)?
                .text(format!("Buon compleanno <@{}>", user.id)))
        } else {
            anyhow::bail!("not a valid user")
        }
    }
}
