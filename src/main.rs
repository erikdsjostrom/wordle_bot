mod command;
mod database;
mod error;
mod player;
mod utils;

use database::Database;
use serenity::model::prelude::command::Command;
use utils::{cup_number_from_unixtime, parse_wordle_msg, recalcualate_high_scores};

use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;

use dotenv::dotenv;
use log::{debug, error, info};

use error::{Error, Result};

use serenity::async_trait;

use serenity::futures::StreamExt;
use serenity::model::application::interaction::Interaction;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::command::score::cup_leader;
use crate::utils::current_cup_number;

// TODO: Make env vars
const CHANNEL_ID: u64 = 938727764037619712;
const GUILD_ID: u64 = 486522741395161108;

struct Bot {
    database: Arc<Database>,
}

struct Scoresheet {
    // TODO
}

// TODO: Factor out
impl Bot {
    async fn set_medals(&self, day: i64, channel_id: ChannelId, http: &Context) -> Result<()> {
        // playerId, msgId, score
        for (p, medalists) in [
            (
                Placement::Gold,
                self.database.get_gold_medalist(Some(day)).await?,
            ),
            (
                Placement::Silver,
                self.database.get_silver_medalist(Some(day)).await?,
            ),
            (
                Placement::Bronze,
                self.database.get_bronze_medalist(Some(day)).await?,
            ),
        ] {
            if medalists.is_none() {
                continue;
            }
            let medalists = medalists.unwrap();
            for (_, msg_id, _) in medalists {
                let msg_id = MessageId(msg_id as u64);
                channel_id
                    .create_reaction(http, msg_id, ReactionType::Unicode(p.to_string()))
                    .await?;
            }
        }
        Ok(())
    }

    async fn clear_medals(&self, day: i64, channel_id: ChannelId, http: &Context) -> Result<()> {
        // playerId, msgId, score
        for (p, medalists) in [
            (
                Placement::Gold,
                self.database.get_gold_medalist(Some(day)).await?,
            ),
            (
                Placement::Silver,
                self.database.get_silver_medalist(Some(day)).await?,
            ),
            (
                Placement::Bronze,
                self.database.get_bronze_medalist(Some(day)).await?,
            ),
        ] {
            if medalists.is_none() {
                continue;
            }
            let medalists = medalists.unwrap();
            for (_, msg_id, _) in medalists {
                let msg_id = MessageId(msg_id as u64);
                channel_id
                    .delete_reaction(http, msg_id, None, ReactionType::Unicode(p.to_string()))
                    .await?;
            }
        }
        Ok(())
    }
    async fn new_score_sheet(&self, msg: &Message) -> Result<()> {
        let cup_number = cup_number_from_unixtime(msg.timestamp.unix_timestamp());
        let player_id = msg.author.id.0 as i64;
        let msg_id = msg.id.0 as i64;
        let score_sheet = msg
            .content
            .strip_prefix("Wordle")
            .ok_or(Error::Message(format!(
                "new_score_sheet called with a non wordle msg: {}",
                msg.content
            )))?;
        // Create new player if not exists
        self.database.new_player(player_id).await?;
        let (day, score) = parse_wordle_msg(score_sheet)?;
        // TODO: Is there a better place to do this to avoid runtime error if this is not executed first?
        self.database.new_daily(day).await?;
        debug!("Day: {}, Score: {}, Cup number: {}", day, score, cup_number);
        self.new_daily_score(Some(day), score).await?;
        self.database
            .new_score_sheet(msg_id, day, player_id, score, cup_number)
            .await?;
        Ok(())
    }

    async fn read_old_messages(&self, channel_id: ChannelId, http: &Context) {
        debug!("Reading old messages");
        let mut msg_count: i64 = 0;
        let mut messages = channel_id.messages_iter(http).boxed();
        while let Some(msg_result) = messages.next().await {
            let msg = match msg_result {
                Err(e) => {
                    error!("{e}");
                    return;
                }
                Ok(msg) => msg,
            };
            if msg.content.starts_with("Wordle") {
                msg_count = msg_count + 1;
                _ = self.new_score_sheet(&msg).await;
            }
        }
        info!("Old messages read: {msg_count}");
    }

    async fn new_daily_score(&self, day: Option<i64>, score: i64) -> Result<()> {
        if score == 0 {
            return Ok(());
        }
        let day = day.unwrap_or(self.database.get_daily_day().await?);
        let high_scores = self.database.get_daily_high_scores(day).await?;
        let high_scores = recalcualate_high_scores(high_scores, score);
        self.database
            .update_daily(day, high_scores[0], high_scores[1], high_scores[2])
            .await
    }

    async fn handle_wordle_message(&self, msg: Message, ctx: &Context) -> Result<()> {
        let score_sheet = msg.content.strip_prefix("Wordle").unwrap();
        let (day, _) = parse_wordle_msg(score_sheet).unwrap();
        self.clear_medals(day, msg.channel_id, &ctx).await?;
        self.new_score_sheet(&msg).await?;
        self.set_medals(day, msg.channel_id, &ctx).await?;
        Ok(())
    }
}

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

#[async_trait]
impl EventHandler for Bot {
    async fn ready(&self, ctx: Context, ready: Ready) {
        debug!("{} is connected!", ready.user.name);

        let guild_id = GuildId(
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
        // .create_application_command(|command| command::kuk::register(command))

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
                // "kuk" => command::kuk::run(&command.data.options).await,
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
            if let Err(err) = self.handle_wordle_message(msg, &ctx).await {
                exit(err);
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
            let message: String = match cup_leader(&current_cup, &database).await {
                Err(err) => exit(err),
                Ok(None) => {
                    error!("No leader in the current cup.");
                    String::from("Ingen vinnare i denna cup.")
                }
                Ok(Some(player)) => {
                    let nick = player.get_nick(guild_id, &ctx.http).await.unwrap();
                    format!("@here Grattis {nick} till vinsten av wordlecupen, du är fan bäst.")
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
