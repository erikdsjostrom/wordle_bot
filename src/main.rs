mod command;
mod database;
mod player;
mod utils;

use database::Database;
use utils::{
    cup_number_from_unixtime, current_cup_number_cute_format, parse_wordle_msg,
    recalcualate_high_scores,
};

use command::stats::stats;

use rand::Rng;
use std::fmt::Write as _;
use std::num::ParseIntError;
use std::sync::Arc;
use std::time::Duration;
use std::{error::Error, fmt::Display};

use dotenv::dotenv;
use log::{debug, error, info};

use serenity::async_trait;

use serenity::futures::StreamExt;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::command::daily::fetch_daily_result;
use crate::command::score::{
    cup_leader, current_cup_leader, current_cup_score, score_since_inception, total_cup_score, FIB,
};
use crate::utils::current_cup_number;

const TESTING_GUILD_ID: u64 = 1065015105437319428;
const CHANNEL_ID: u64 = 938727764037619712;
const GUILD_ID: u64 = 486522741395161108;

type Result<T> = std::result::Result<T, WordleError>;

#[derive(Debug)]
enum WordleError {
    IlleagalNumberOfGuesses(char),
    ParseIntError(ParseIntError),
    ParseError(String),
}

impl From<ParseIntError> for WordleError {
    fn from(v: ParseIntError) -> Self {
        Self::ParseError(v.to_string())
    }
}

impl Display for WordleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WordleError::IlleagalNumberOfGuesses(x) => {
                f.write_fmt(format_args!("Illeagal number of guesses: {x}"))
            }
            WordleError::ParseIntError(s) => s.fmt(f),
            WordleError::ParseError(s) => f.write_str(&s),
        }
    }
}
impl Error for WordleError {}

struct Bot {
    database: Arc<Database>,
}

impl Bot {
    async fn set_status_to_current_leader(&self, ctx: &Context) {
        if let Some(player) = current_cup_leader(&self.database).await {
            let nick = player.get_nick(GUILD_ID.into(), ctx).await.unwrap();
            let msg = format!("Nuvarande cupledare: {nick}");
            ctx.set_activity(Activity::competing(msg)).await;
        } else {
            ctx.set_presence(None, OnlineStatus::Idle).await;
        };
    }

    async fn set_medals(&self, day: i64, channel_id: ChannelId, http: &Context) {
        // playerId, msgId, score
        for (p, medalists) in [
            (
                Placement::Gold,
                self.database.get_gold_medalist(Some(day)).await,
            ),
            (
                Placement::Silver,
                self.database.get_silver_medalist(Some(day)).await,
            ),
            (
                Placement::Bronze,
                self.database.get_bronze_medalist(Some(day)).await,
            ),
        ] {
            if medalists.is_none() {
                continue;
            }
            let medalists = medalists.unwrap();
            for (_, msg_id, _) in medalists {
                let msg_id = MessageId(msg_id as u64);
                let res = channel_id
                    .create_reaction(http, msg_id, ReactionType::Unicode(p.to_string()))
                    .await;
                match res {
                    Ok(_) => (),
                    Err(e) => error!("{e}"),
                }
            }
        }
    }

    async fn clear_medals(&self, day: i64, channel_id: ChannelId, http: &Context) {
        // playerId, msgId, score
        for (p, medalists) in [
            (
                Placement::Gold,
                self.database.get_gold_medalist(Some(day)).await,
            ),
            (
                Placement::Silver,
                self.database.get_silver_medalist(Some(day)).await,
            ),
            (
                Placement::Bronze,
                self.database.get_bronze_medalist(Some(day)).await,
            ),
        ] {
            if medalists.is_none() {
                continue;
            }
            let medalists = medalists.unwrap();
            for (_, msg_id, _) in medalists {
                let msg_id = MessageId(msg_id as u64);
                let res = channel_id
                    .delete_reaction(http, msg_id, None, ReactionType::Unicode(p.to_string()))
                    .await;
                match res {
                    Ok(_) => (),
                    Err(e) => error!("{e}"),
                }
            }
        }
    }
    // TODO: This return type is horrible
    async fn new_score_sheet(&self, msg: &Message) -> Option<()> {
        let cup_number = cup_number_from_unixtime(msg.timestamp.unix_timestamp());
        let player_id = msg.author.id.0 as i64;
        let msg_id = msg.id.0 as i64;
        let score_sheet = msg.content.strip_prefix("Wordle").unwrap();
        // Create new player if not exists
        self.database.new_player(player_id).await;
        let (day, score) = match parse_wordle_msg(score_sheet) {
            Ok(x) => x,
            Err(e) => {
                error!("{e}");
                return None;
            }
        };
        // TODO: Is there a better place to do this to avoid runtime error if this is not executed first?
        self.database.new_daily(day).await;
        debug!("Day: {}, Score: {}, Cup number: {}", day, score, cup_number);
        self.new_daily_score(Some(day), score).await;
        self.database
            .new_score_sheet(msg_id, day, player_id, score, cup_number)
            .await;
        Some(())
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

    async fn new_daily_score(&self, day: Option<i64>, score: i64) {
        if score == 0 {
            return;
        }
        let day = day.unwrap_or(self.database.get_daily_day().await);
        let high_scores = self.database.get_daily_high_scores(day).await;
        let high_scores = recalcualate_high_scores(high_scores, score);
        self.database
            .update_daily(day, high_scores[0], high_scores[1], high_scores[2])
            .await;
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
        let database = Arc::clone(&self.database);
        tokio::spawn(async move {
            check_if_cup_winner(ctx.clone(), Arc::clone(&database)).await;
        });
    }

    async fn cache_ready(&self, _ctx: Context, _guilds: Vec<GuildId>) {
        debug!("Cache ready.");
    }

    async fn message(&self, ctx: Context, msg: Message) {
        let _is_testing: bool = match msg.guild_id {
            Some(guild_id) => {
                if guild_id.0 == TESTING_GUILD_ID {
                    debug!("Msg from test server");
                    true
                } else {
                    debug!("Guild id: {guild_id}");
                    false
                }
            }
            None => {
                debug!("No guild id");
                false
            }
        };
        if msg.content == "!stats" {
            let response = stats(msg.author.id.0 as i64, &self.database).await;
            msg.channel_id.say(&ctx, response).await.unwrap();
        }
        if msg.content.starts_with("Wordle") {
            let score_sheet = msg.content.strip_prefix("Wordle").unwrap();
            let (day, _) = parse_wordle_msg(score_sheet).unwrap();
            self.clear_medals(day, msg.channel_id, &ctx).await;
            _ = self.new_score_sheet(&msg).await;
            self.set_medals(day, msg.channel_id, &ctx).await;
            self.set_status_to_current_leader(&ctx).await;
        } else if msg.content.eq("!ställning alt") {
            let score = score_since_inception(&self.database).await;
            let mut response = "Ställning i världscupen:\n".to_string();
            for (player, result) in score {
                let user = player
                    .get_nick(GUILD_ID.into(), &ctx.http)
                    .await
                    .unwrap_or(String::from("Unknown user"));
                writeln!(response, "\t{user}: {result}").unwrap();
            }
            msg.channel_id.say(&ctx, response).await.unwrap();
        } else if msg.content.eq("!ställning") {
            let score = current_cup_score(&self.database).await;
            let cup_number = current_cup_number_cute_format();
            let mut response = format!("Ställning i månadscupen {}:\n", cup_number);
            for (player, result) in score {
                let user = player
                    .get_nick(GUILD_ID.into(), &ctx.http)
                    .await
                    .unwrap_or(String::from("Unknown user"));
                writeln!(response, "\t{user}: {result}").unwrap();
            }
            msg.channel_id.say(&ctx, response).await.unwrap();
        } else if msg.content.eq("!ställning totala") {
            let score = total_cup_score(&self.database).await;
            let mut response = "Ställning i totala världscupen:\n".to_string();
            for (player, result) in score {
                let user = player
                    .get_nick(GUILD_ID.into(), &ctx.http)
                    .await
                    .unwrap_or(String::from("Unknown user"));
                writeln!(response, "\t{user}: {result}").unwrap();
            }
            msg.channel_id.say(&ctx, response).await.unwrap();
        } else if msg.content.starts_with("!dagens") {
            debug!("!dagens");
            let daily = fetch_daily_result(&self.database).await;
            let mut response = String::from("Dagens placering:\n");
            for (placement, medalists) in daily {
                if medalists.is_none() {
                    break;
                }
                let medalists = medalists.unwrap();
                let mut users = String::default();
                for player in medalists.0 {
                    let nick = player.get_nick(GUILD_ID.into(), &ctx.http).await.unwrap();
                    if !users.is_empty() {
                        users.push_str(", ");
                    }
                    users.push_str(&nick);
                }
                // XXX
                _ = writeln!(
                    response,
                    "{} - {} - {} försök ({}p)",
                    placement.to_string(),
                    users,
                    medalists.1,
                    FIB[medalists.1 as usize]
                );
            }
            msg.channel_id.say(&ctx, response).await.unwrap();
        } else if msg.content == "!hurlångkukharjonathancissig" {
            let kuk_svar = match rand::thread_rng().gen_range(1..11) {
                10 => format!("Vilken kuk?"),
                x => format!("Jonathan Cissigs kuk är {x} cm lång."),
            };
            msg.channel_id.say(&ctx, kuk_svar).await.unwrap();
        } else if msg.content == "!hurlångkukharaxelbolle" {
            let kuk_svar = match rand::thread_rng().gen_range(1..11) {
                x => format!("Carl Vilhelm Alex Bolles kuk är {x} cm långsmal."),
            };
            msg.channel_id.say(&ctx, kuk_svar).await.unwrap();
        } else if msg.content == "!hurlångkukharmikaelösterdahl" {
            let kuk_svar = match rand::thread_rng().gen_range(1..11) {
                10 => format!("Grisens kuk pekar inåt."),
                x => format!("Mikael Österdahls kuk är {x} cm lång."),
            };
            msg.channel_id.say(&ctx, kuk_svar).await.unwrap();
        } else if msg.content == "!hurlångkukharlinusågren" {
            let kuk_svar = match rand::thread_rng().gen_range(1..11) {
                10 => format!("Varför är du så jävla intresserad av kuklängd?."),
                x => format!("Linus Ågrens kuk är {x} cm lång."),
            };
            msg.channel_id.say(&ctx, kuk_svar).await.unwrap();
        } else if msg.content == "!hurlångkukhareriksjöström" {
            let kuk_svar = match rand::thread_rng().gen_range(1..11) {
                10 => format!("Större än din."),
                x => {
                    let x = 10 + x;
                    format!("Erik Sjöströms kuk är {x} cm lång.")
                }
            };
            msg.channel_id.say(&ctx, kuk_svar).await.unwrap();
        // Admin messages - check for privilege, then execute and delete message
        } else if msg.content == "!debug" {
            if msg.author.name != "esjostrom" {
                msg.channel_id
                    .say(&ctx, "Du är ej betrodd med detta kommando")
                    .await
                    .unwrap();
                return;
            }
            let channel_id = msg.channel_id.0;
            let guild_id = msg.guild_id.unwrap().0;
            let cup_leader = current_cup_leader(&self.database)
                .await
                .unwrap()
                .get_nick(GUILD_ID.into(), &ctx.http)
                .await
                .unwrap();
            let mut message = String::from("Debug info:\n");
            writeln!(message, "Cup leader: {cup_leader}").unwrap();
            msg.channel_id.say(&ctx, message).await.unwrap();
            debug!("Channel id: {channel_id}, Guild id: {guild_id}");
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

// Checks if we have entered a new cup season, if so prints of the winner of the last cup.
async fn check_if_cup_winner(ctx: Context, database: Arc<Database>) {
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
                None => {
                    error!("No leader in the current cup.");
                    String::from("Ingen vinnare i denna cup.")
                }
                Some(player) => {
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

    // Initiate a connection to the database file, creating the file if required.
    let database = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename("database.sqlite")
                .create_if_missing(true),
        )
        .await
        .expect("Couldn't connect to database");

    // Run migrations, which updates the database's schema to the latest version.
    sqlx::migrate!("./migrations")
        .run(&database)
        .await
        .expect("Couldn't run database migrations");

    let database = Database::new("database.sqlite").await;

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
