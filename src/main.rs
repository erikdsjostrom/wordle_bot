mod database;
use database::Database;

use rand::Rng;
use std::cmp::Ordering;
use std::fmt::Write as _;
use std::num::ParseIntError;
use std::{collections::HashMap, error::Error, fmt::Display};

use dotenv::dotenv;
use log::{debug, error, info};

use serenity::async_trait;

use serenity::http::CacheHttp;
use serenity::model::prelude::*;
use serenity::prelude::*;

const TESTING_GUILD_ID: u64 = 1065015105437319428;
const DAY_OF_INCEPTION: i64 = 580;

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
    database: Database,
}

// Assumes sorted input
fn recalcualate_high_scores(high_scores: [Option<i64>; 3], score: i64) -> [Option<i64>; 3] {
    match (high_scores[0], high_scores[1], high_scores[2]) {
        (None, None, None) => [Some(score), None, None],
        (Some(gold), None, None) => match gold.cmp(&score) {
            Ordering::Greater => [Some(score), Some(gold), None],
            Ordering::Equal => [Some(gold), None, None],
            Ordering::Less => [Some(gold), Some(score), None],
        },
        (Some(gold), Some(silver), None) => match (gold.cmp(&score), silver.cmp(&score)) {
            (Ordering::Greater, _) => [Some(score), Some(gold), Some(silver)],
            (Ordering::Equal, _) => [Some(gold), Some(silver), None],
            (Ordering::Less, Ordering::Greater) => [Some(gold), Some(score), Some(silver)],
            (Ordering::Less, Ordering::Equal) => [Some(gold), Some(silver), None],
            (Ordering::Less, Ordering::Less) => [Some(gold), Some(silver), Some(score)],
        },
        (Some(gold), Some(silver), Some(bronze)) => {
            match (gold.cmp(&score), silver.cmp(&score), bronze.cmp(&score)) {
                (Ordering::Greater, _, _) => [Some(score), Some(gold), Some(silver)],
                (Ordering::Equal, _, _) => [Some(gold), Some(silver), Some(bronze)],
                (Ordering::Less, Ordering::Equal, _) => [Some(gold), Some(silver), Some(bronze)],
                (Ordering::Less, Ordering::Greater, _) => [Some(gold), Some(score), Some(silver)],
                (Ordering::Less, Ordering::Less, Ordering::Less) => {
                    [Some(gold), Some(silver), Some(bronze)]
                }
                (Ordering::Less, Ordering::Less, Ordering::Equal) => {
                    [Some(gold), Some(silver), Some(bronze)]
                }
                (Ordering::Less, Ordering::Less, Ordering::Greater) => {
                    [Some(gold), Some(silver), Some(score)]
                }
            }
        }
        _ => unreachable!(),
    }
}

impl Bot {
    async fn stats(&self, user_id: i64) -> String {
        let mut response: String = String::from("");
        let gold_medals = self.get_user_medals(Placement::Gold, user_id).await;
        let silver_medals = self.get_user_medals(Placement::Silver, user_id).await;
        let bronze_medals = self.get_user_medals(Placement::Bronze, user_id).await;
        let played_games = self.database.get_user_played_games(user_id).await;

        writeln!(response, "Antal spelade spel: **{}**", played_games).unwrap();
        writeln!(
            response,
            "{} medaljer: **{}**",
            Placement::Gold.to_string(),
            gold_medals
        )
        .unwrap();
        writeln!(
            response,
            "{} medaljer: **{}**",
            Placement::Silver.to_string(),
            silver_medals
        )
        .unwrap();
        writeln!(
            response,
            "{} medaljer: **{}**",
            Placement::Bronze.to_string(),
            bronze_medals
        )
        .unwrap();

        writeln!(response, "").unwrap();

        let scores = self.database.get_user_scores(user_id).await;
        let total: f64 = scores.len() as f64;
        let mut score_count: HashMap<i64, f64> = HashMap::from([
            (0, 0.0),
            (1, 0.0),
            (2, 0.0),
            (3, 0.0),
            (4, 0.0),
            (5, 0.0),
            (6, 0.0),
        ]);
        dbg!(&scores);
        for score in scores {
            *score_count.get_mut(&score).unwrap() += 1.0;
        }
        dbg!(&score_count);
        writeln!(response, "Poängfördelning:").unwrap();
        for score in [0, 1, 2, 3, 4, 5, 6] {
            let ratio = 100.0 * score_count[&score] / total;
            writeln!(
                response,
                "{}\t|\t{}|\t{}",
                score.to_string(),
                "█".repeat(ratio as usize),
                score_count[&score]
            )
            .unwrap();
        }

        response
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
        debug!("Day: {}, Score: {}", day, score);
        self.new_daily_score(Some(day), score).await;
        self.database
            .new_score_sheet(msg_id, day, player_id, score)
            .await;
        Some(())
    }

    // TODO: Run on connect, exit when encounter day already in database
    // A continious application of GetMessages should do the trick
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

    async fn get_user_medals(&self, medal: Placement, user_id: i64) -> i32 {
        match medal {
            Placement::Gold => self.database.get_user_gold_medals(user_id).await,
            Placement::Silver => self.database.get_user_silver_medals(user_id).await,
            Placement::Bronze => self.database.get_user_bronze_medals(user_id).await,
            Placement::Loser => panic!(),
        }
    }

    async fn new_daily_score(&self, day: Option<i64>, score: i64) {
        if score == 0 {
            return;
        }
        let day = day.unwrap_or(self.database.get_daily_day().await);
        let high_scores = self.database.get_daily_high_scores(day).await;
        let high_scores = recalcualate_high_scores(high_scores, score);
        self
            .database
            .update_daily(day, high_scores[0], high_scores[1], high_scores[2])
            .await;
    }
    // Calculate the current score of all players
    // The wordle scores are used as indexes into the fibonachi sequence
    // to lend more weight to earlier correct guesses
    // TODO: Result return type
    async fn score(&self, total: bool, cache: &impl CacheHttp, guild_id: GuildId) -> String {
        let from_day = {
            if total {
                0
            } else {
                DAY_OF_INCEPTION
            }
        };
        let mut leader_board: Vec<(String, u32)> = vec![];
        // Failure (X) gives a score of zero
        let fib: [u32; 7] = [0, 13, 8, 5, 3, 2, 1];
        let players = &self.database.get_players().await;
        for player_id in players {
            let user = get_nick(*player_id, guild_id, cache).await;
            let scores = &self
                .database
                .get_player_scores_from_day(from_day, *player_id)
                .await;
            let mut result: u32 = 0;
            for score in scores {
                result = result + fib[*score as usize];
            }
            leader_board.push((user, result));
        }
        leader_board.sort_by_key(|x| x.1);
        leader_board.reverse();
        let header = {
            if total {
                "Ställning i totala världscupen:".to_string()
            } else {
                "Ställning i världscupen:".to_string()
            }
        };
        let response: String = leader_board.iter().fold(header, |acc, (user, score)| {
            format!("{acc}\n\t{user}: {score}")
        });
        response
    }
}

fn graph_scores(scores: Vec<i64>) -> String {
    let total: f64 = scores.len() as f64;
    let mut score_count: HashMap<i64, f64> = HashMap::from([
        (0, 0.0),
        (1, 0.0),
        (2, 0.0),
        (3, 0.0),
        (4, 0.0),
        (5, 0.0),
        (6, 0.0),
    ]);
    dbg!(&scores);
    for score in scores {
        *score_count.get_mut(&score).unwrap() += 1.0;
    }
    dbg!(&score_count);
    let mut graph = String::from("Poängfördelning:\n");
    for score in [0, 1, 2, 3, 4, 5, 6] {
        let ratio = 100.0 * score_count[&score] / total;
        dbg!(&ratio);
        dbg!("#".repeat(ratio as usize));
        writeln!(
            graph,
            "{} | {}",
            score.to_string(),
            "#".repeat(ratio as usize)
        )
        .unwrap();
    }
    graph
}

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
enum Placement {
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

// Returns wordle number and score
fn parse_wordle_msg(msg: &str) -> Result<(i64, i64)> {
    debug!("{msg}");
    let mut msg = msg.trim().split(' ');
    let wordle_number: i64 = match msg.next() {
        Some(it) => it,
        None => {
            return Err(WordleError::ParseError(
                "Wordle number not found while parsing.".to_string(),
            ))
        }
    }
    .parse()?;
    let score: i64 = match msg
        .next()
        .ok_or(WordleError::ParseError(String::from(
            "Wordle message does not contain a score.",
        )))?
        .chars()
        .next()
        .ok_or(WordleError::ParseError(String::from(
            "Unable to split score into chars.",
        )))? {
        'X' | '0' => 0, // Ok, since there's no 0 guess score
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        x => return Err(WordleError::IlleagalNumberOfGuesses(x)),
    };
    Ok((wordle_number, score))
}

/// Tries to get the nick of the user, if it fails returns the user name
async fn get_nick(user_id: i64, guild_id: GuildId, cache: &impl CacheHttp) -> String {
    debug!("User id: {user_id}");
    let user_id = UserId(user_id as u64);
    let user = match user_id.to_user(cache).await {
        Ok(user) => user,
        Err(e) => {
            error!("Error: {e}");
            return String::from("Unknown user");
        }
    };
    if let Some(nick) = user.nick_in(cache, guild_id).await {
        nick
    } else {
        debug!("No nick found for user: {}", user.name);
        user.name
    }
}

use serenity::futures::StreamExt;

impl Placement {
    fn dec(&self) -> Placement {
        match self {
            Placement::Gold => Placement::Silver,
            Placement::Silver => Placement::Bronze,
            _ => Placement::Loser,
        }
    }
}

// Assumes sorted input
fn calculate_placements(
    mut scores: Vec<(i64, (i64, i64))>,
) -> HashMap<Placement, (i64, Vec<(i64, i64)>)> {
    dbg!(&scores);
    let mut placements: HashMap<Placement, (i64, Vec<(i64, i64)>)> = HashMap::new();
    let mut current_score: i64 = 10;
    let mut current_placement: Placement = Placement::Gold;
    scores.sort_by_key(|x| x.0);
    dbg!(&scores);
    for (score, id) in scores {
        if score == 0 {
            todo!();
        }
        match (
            placements.get(&Placement::Gold),
            placements.get(&Placement::Silver),
            placements.get(&Placement::Bronze),
        ) {
            (None, _, _) => _ = placements.insert(Placement::Gold, (score, vec![id])),
            (Some(x @ (gold_score, _)), None, None) => match score.cmp(&gold_score) {
                Ordering::Less => _ = placements.insert(Placement::Silver, (score, vec![id])),
                Ordering::Equal => placements.get_mut(&Placement::Gold).unwrap().1.push(id),
                Ordering::Greater => {
                    placements.insert(Placement::Silver, x.clone());
                    placements.insert(Placement::Gold, (score, vec![id]));
                }
            },
            (Some(x @ (gold_score, _)), Some(y @ (silver_score, _)), None) => {
                match (score.cmp(&gold_score), score.cmp(&silver_score)) {
                    (Ordering::Less, Ordering::Less) => {
                        _ = placements.insert(Placement::Bronze, (score, vec![id]))
                    }
                    (Ordering::Less, Ordering::Equal) => {
                        placements.get_mut(&Placement::Silver).unwrap().1.push(id)
                    }
                    (Ordering::Less, Ordering::Greater) => {
                        placements.insert(Placement::Bronze, y.clone());
                        placements.insert(Placement::Silver, (score, vec![id]));
                    }
                    (Ordering::Equal, _) => {
                        placements.get_mut(&Placement::Gold).unwrap().1.push(id)
                    }
                    (Ordering::Greater, _) => {
                        // placements.insert(Placement::Bronze, y.clone());
                        // placements.insert(Placement::Silver, x.clone());
                        // placements.insert(Placement::Gold, (score, vec![id]));
                        todo!();
                    }
                }
            }
            (Some(_), Some(_), Some(_)) => todo!(),
            _ => unreachable!(),
        };
        // Don't give losers gold medals
        if score == 0 {
            if let Some(v) = placements.get_mut(&Placement::Loser) {
                v.1.push(id);
            } else {
                placements.insert(Placement::Loser, (score, vec![id]));
            }
            continue;
        } else if placements.is_empty() {
            current_placement = Placement::Gold;
        } else if score != current_score {
            current_placement = current_placement.dec();
        }
        current_score = score;
        if let Some(v) = placements.get_mut(&current_placement) {
            v.1.push(id);
        } else {
            placements.insert(current_placement, (score, vec![id]));
        }
    }
    placements
}

#[async_trait]
impl EventHandler for Bot {
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
            let response = self.stats(msg.author.id.0 as i64).await;
            msg.channel_id.say(&ctx, response).await.unwrap();
        }
        if msg.content.starts_with("Wordle") {
            _ = self.new_score_sheet(&msg).await;
            let score_sheet = msg.content.strip_prefix("Wordle").unwrap();
            let (day, score) = parse_wordle_msg(score_sheet).unwrap();
            self.clear_medals(day, msg.channel_id, &ctx).await;
            self.new_daily_score(Some(day), score).await;
            self.set_medals(day, msg.channel_id, &ctx).await;
        } else if msg.content.eq("!ställning") {
            debug!("!ställning");
            let response = &self
                .score(false, &ctx.http, msg.guild_id.unwrap().into())
                .await;
            msg.channel_id.say(&ctx, response).await.unwrap();
        } else if msg.content.eq("!ställning totala") {
            debug!("!ställning");
            let response = &self
                .score(true, &ctx.http, msg.guild_id.unwrap().into())
                .await;
            msg.channel_id.say(&ctx, response).await.unwrap();
        } else if msg.content.starts_with("!dagens") {
            debug!("!dagens");
            let mut response = String::from("Dagens placering:\n");
            for (placement, medalists) in [
                (Placement::Gold, self.database.get_gold_medalist(None).await),
                (
                    Placement::Silver,
                    self.database.get_silver_medalist(None).await,
                ),
                (
                    Placement::Bronze,
                    self.database.get_bronze_medalist(None).await,
                ),
            ] {
                if medalists.is_none() {
                    continue;
                }
                let medalists = medalists.unwrap();
                let mut user_list = vec![];
                let mut score = 0;
                for (player_id, _, new_score) in medalists {
                    score = new_score;
                    let ass = get_nick(player_id, msg.guild_id.unwrap(), &ctx.http).await;
                    user_list.push(ass);
                }
                let mut users = String::from("");
                let mut iter = user_list.into_iter().peekable();
                while let Some(user) = iter.next() {
                    users.push_str(&user);
                    if iter.peek().is_some() {
                        users.push_str(", ");
                    }
                }
                // XXX
                _ = writeln!(
                    response,
                    "{} - {} - {} försök",
                    placement.to_string(),
                    users,
                    score
                );
            }
            msg.channel_id.say(&ctx, response).await.unwrap();
        } else if msg.content == "!hurlångkukharjonathancissig" {
            let kuk_svar = match rand::thread_rng().gen_range(1..11) {
                10 => format!("Vilken kuk?"),
                9 => format!("runtime error: unsigned integer underflow: jompis_kuk.length() cannot be represented in type 'uint'"),
                x => format!("Jonathan Cissigs kuk är {x} cm lång."),
            };
            msg.channel_id.say(&ctx, kuk_svar).await.unwrap();
        } else if msg.content == "!hurlångkukharbolle" {
            let kuk_svar = match rand::thread_rng().gen_range(1..11) {
                x => format!("Carl Vilhelm Alex Bolles kuk är {x} cm långsmal."),
            };
            msg.channel_id.say(&ctx, kuk_svar).await.unwrap();
        } else if msg.content == "!hurlångkukharmikael" {
            let kuk_svar = match rand::thread_rng().gen_range(1..11) {
                _ => format!("Grisens kuk pekar inåt."),
            };
            msg.channel_id.say(&ctx, kuk_svar).await.unwrap();
        } else if msg.content == "!hurlångkukharlinus" {
            let kuk_svar = match rand::thread_rng().gen_range(1..11) {
                _ => format!("Varför är du så jävla intresserad av kuklängd?."),
            };
            msg.channel_id.say(&ctx, kuk_svar).await.unwrap();
        } else if msg.content == "!hurlångkukharerik" {
            let kuk_svar = match rand::thread_rng().gen_range(1..11) {
                _ => format!("Större än din."),
            };
            msg.channel_id.say(&ctx, kuk_svar).await.unwrap();
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
        } else if msg.content == "!medaljera" {
            if msg.author.name != "esjostrom" {
                msg.channel_id
                    .say(&ctx, "Du är ej betrodd med detta kommando")
                    .await
                    .unwrap();
                return;
            }
            // set_medals(msg.channel_id, &self.database, &ctx).await;
        }
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

    let bot = Bot { database };

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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_recalculate_scores() {
        assert_eq!(
            recalcualate_high_scores([None, None, None], 1),
            [Some(1), None, None]
        );

        assert_eq!(
            recalcualate_high_scores([Some(1), None, None], 1),
            [Some(1), None, None]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), None, None], 1),
            [Some(1), Some(2), None]
        );

        assert_eq!(
            recalcualate_high_scores([Some(1), None, None], 2),
            [Some(1), Some(2), None]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), None], 6),
            [Some(2), Some(4), Some(6)]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), None], 1),
            [Some(1), Some(2), Some(4)]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), None], 3),
            [Some(2), Some(3), Some(4)]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), None], 2),
            [Some(2), Some(4), None]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), None], 4),
            [Some(2), Some(4), None]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), Some(6)], 5),
            [Some(2), Some(4), Some(5)]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), Some(6)], 6),
            [Some(2), Some(4), Some(6)]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), Some(6)], 7),
            [Some(2), Some(4), Some(6)]
        );
    }
}
