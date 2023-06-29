mod bot;
mod command;
mod database;
mod error;
mod parser;
mod player;
mod scoresheet;
mod utils;

use std::{fmt::Display, sync::Arc, time::Duration};

use bot::Bot;
use database::Database;
use dotenv::dotenv;
use error::{Error, Result};
use log::{debug, error, info};
use rand::seq::IteratorRandom;
use serenity::{
    async_trait,
    model::{
        application::interaction::{Interaction, InteractionResponseType},
        prelude::{command::Command, *},
    },
    prelude::*,
};

use crate::utils::current_cup_number;

// TODO: Make env vars
const CHANNEL_ID: u64 = 938727764037619712;
const GUILD_ID: u64 = 486522741395161108;
const CONGRATULATIONS: [&str; 10] = [
        "@here Grattis {nick} till segern i wordlecupen, du är verkligen bäst!",
        "@here Stort grattis till {nick} som vann wordlecupen, du är en riktig mästare!",
        "@here Wow! Grattis {nick} till att ha erövrat wordlecupen, du är grym!",
        "@here Fantastiskt jobbat, {nick}! Du är en vinnare och tar hem wordlecupen med bravur!",
        "@here Grattis, {nick}! Du har lyckats bli mästaren i wordlecupen, en värdig vinnare!",
        "@here Stort grattis till {nick} för att ha vunnit wordlecupen, du är en riktig mästare!",
        "@here Fantastiskt jobbat, {nick}! Du har tagit hem segern i wordlecupen, du är grymt bra!",
        "@here Grattis, {nick}! Ditt framstående spel har belönats med vinsten i wordlecupen, du är verkligen bäst!",
        "@here Wow! {nick}, du är en riktig vinnare som har erövrat wordlecupen. Stort grattis!",
        "@here Enorma gratulationer till {nick} för att ha segrat i wordlecupen. Du är en otroligt skicklig spelare!"
    ];

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub enum Placement {
    Gold,
    Silver,
    Bronze,
    Loser,
}

impl Display for Placement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Placement::Gold => f.write_str("🥇"),
            Placement::Silver => f.write_str("🥈"),
            Placement::Bronze => f.write_str("🥉"),
            Placement::Loser => f.write_str(":youtried:"),
        }
    }
}

// XXX
async fn update_channel_title(
    channel: &mut GuildChannel,
    database: &Database,
    ctx: &Context,
) -> Result<()> {
    let dagens_ledare = match database.get_gold_medalist(None).await? {
        Some(players) => {
            let mut leaders: Vec<String> = Vec::default();
            for player in players {
                leaders.push(player.player().get_nick(channel.guild_id, ctx).await?);
            }
            leaders.join(", ")
        }
        None => "Tomten".to_string(),
    };
    let cup_ledare = match database.current_cup_leader().await? {
        Some(player) => player.get_nick(channel.guild_id, ctx).await?,
        None => "Tomten".to_string(),
    };
    let title = format!("Dagens ledare: {dagens_ledare}\tCupledare: {cup_ledare}");
    channel
        .edit(ctx, |c| c.topic(title))
        .await
        .map_err(|err| err.into())
}

#[async_trait]
impl EventHandler for Bot {
    async fn ready(&self, ctx: Context, ready: Ready) {
        debug!("{} is connected!", ready.user.name);

        let _guild_id = GuildId(
            std::env::var("GUILD_ID")
                .expect("Expected GUILD_ID in environment")
                .parse()
                .expect("GUILD_ID must be an integer"),
        );

        // Commands can be global as well
        let commands = [
            Command::create_global_application_command(&ctx.http, |command| {
                command::score::register(command)
            })
            .await,
            Command::create_global_application_command(&ctx.http, |command| {
                command::daily::register(command)
            })
            .await,
            Command::create_global_application_command(&ctx.http, |command| {
                command::stats::register(command)
            })
            .await,
        ];

        debug!(
            "I now have the following global slash commands: {:#?}",
            commands
        );

        let database = Arc::clone(&self.database);
        tokio::spawn(async move {
            check_for_cup_winner(ctx.clone(), Arc::clone(&database)).await;
        });
    }

    async fn cache_ready(&self, _ctx: Context, _guilds: Vec<GuildId>) {
        debug!("Cache ready.");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            debug!("Received command interaction: {:#?}", command);

            let content = match command.data.name.as_str() {
                "ställning" => {
                    command::score::run(&self.database, &ctx, &command.data.options).await
                }
                "dagens" => command::daily::run(&self.database, &ctx, &command.data.options).await,
                "stats" => {
                    command::stats::run(
                        &command.user.id.into(),
                        &self.database,
                        &command.data.options,
                    )
                    .await
                }
                _ => Err(Error::UnknownCommand),
            };

            let content = match content {
                Ok(c) => c,
                Err(e) => exit(e),
            };

            if let Err(why) = command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| message.content(content))
                })
                .await
            {
                debug!("Cannot respond to slash command: {}", why);
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content.starts_with("Wordle") {
            if let Err(err) = self.handle_wordle_message(&msg, &ctx).await {
                exit(err);
            }
            if let Ok(channel) = msg.channel(&ctx).await {
                if let Some(mut guild_channel) = channel.guild() {
                    match update_channel_title(&mut guild_channel, &self.database, &ctx).await {
                        Ok(_) => (),
                        Err(e) => error!("{e}"),
                    };
                }
            }
        // Admin messages - check for privilege, then execute and delete message
        } else if msg.content == "!reset" {
            if msg.author.name != "esjostrom" {
                msg.channel_id
                    .say(&ctx, "Du är ej betrodd med detta kommando")
                    .await
                    .unwrap();
                return;
            }
            self.read_old_messages(msg.channel_id, &ctx).await;
        }
    }
}

fn exit(err: Error) -> ! {
    error!("{err}");
    std::process::exit(1);
}

// Checks if we have entered a new cup season, if so prints of the winner of the last cup.
async fn check_for_cup_winner(ctx: Context, database: Arc<Database>) {
    debug!("Cup winner check loop spawned.");
    // Run check every two hours
    let mut interval = tokio::time::interval(Duration::from_secs(2 * 60 * 60));
    let mut current_cup = current_cup_number();
    let channel_id: ChannelId = CHANNEL_ID.into();
    let guild_id: GuildId = GUILD_ID.into();
    loop {
        let cup = current_cup_number();
        if current_cup != cup {
            let message: String = match database.cup_leader(&current_cup).await {
                Err(err) => exit(err),
                Ok(None) => {
                    error!("No leader in the current cup.");
                    String::from("Ingen vinnare i denna cup.")
                }
                Ok(Some(player)) => {
                    let nick = player.get_nick(guild_id, &ctx.http).await.unwrap();
                    CONGRATULATIONS
                        .iter()
                        .choose(&mut rand::thread_rng())
                        .unwrap()
                        .replace("{nick}", &nick)
                }
            };
            channel_id.say(&ctx, message).await.unwrap();
            info!("Cup winner announced");
            current_cup = cup;
        }
        debug!("No new cup winner for cup {current_cup}.");
        interval.tick().await;
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv().ok();
    // Configure the client with your Discord bot token in the environment.
    let token = std::env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let database = match Database::new("database.sqlite").await {
        Ok(db) => db,
        Err(err) => {
            error!("{err}");
            std::process::exit(1);
        }
    };

    let bot = Bot {
        database: Arc::new(database),
    };

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(&token, intents)
        .event_handler(bot)
        .await
        .expect("Err creating client");
    match client.start().await {
        Ok(_) => (),
        Err(e) => eprintln!("{e}"),
    }
}
