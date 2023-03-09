use std::fmt::Write as _;
use std::future::Future;

use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::interaction::application_command::CommandDataOption;
use serenity::prelude::*;

use log::debug;

use crate::{
    database::Database, error::Result, player::Player, utils::current_cup_number_cute_format,
    GUILD_ID,
};

// Failure (X) gives a score of zero
pub const FIB: [u32; 7] = [0, 13, 8, 5, 3, 2, 1];

pub async fn run(
    database: &Database,
    ctx: &Context,
    _options: &[CommandDataOption],
) -> Result<String> {
    let score = current_cup_score(database).await?;
    let cup_number = current_cup_number_cute_format();
    let mut response = format!("Ställning i månadscupen {}:\n", cup_number);
    for (player, result) in score {
        let user = player.get_nick(GUILD_ID.into(), &ctx.http).await?;
        writeln!(response, "\t{user}: {result}").unwrap();
    }
    Ok(response)
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("ställning")
        .description("Nuvarande ställning i månadscupen.")
}

// Calculate the current score of all players
// The wordle scores are used as indexes into the fibonachi sequence
// to lend more weight to earlier correct guesses
// TODO: Factor out database to make it testable
async fn calculate_leader_board<F, Fut>(
    database: &Database,
    score_getter: F,
) -> Result<Vec<(Player, u32)>>
where
    F: Fn(i64) -> Fut,
    Fut: Future<Output = Result<Vec<i64>>>,
{
    debug!("Calculating leader board");
    let mut fetched: usize = 0;
    let mut leader_board: Vec<(Player, u32)> = vec![];
    let players = database.get_players().await?;
    for player_id in players {
        let scores = score_getter(player_id).await?;
        fetched += scores.len();
        let score = scores.iter().fold(0, |acc, elem| acc + FIB[*elem as usize]);
        if score == 0 {
            continue;
        }
        leader_board.push((Player::from(player_id), score));
    }
    debug!("Fetched {fetched} score sheets.");
    // XXX
    leader_board.sort_by_key(|x| x.1);
    leader_board.reverse();
    Ok(leader_board)
}

pub async fn total_cup_score(database: &Database) -> Result<Vec<(Player, u32)>> {
    let score_getter =
        |player_id| async move { database.get_player_scores_from_day(0, player_id).await };
    calculate_leader_board(database, score_getter).await
}

pub async fn current_cup_score(database: &Database) -> Result<Vec<(Player, u32)>> {
    let score_getter =
        |player_id| async move { database.get_player_scores_for_current_cup(player_id).await };
    calculate_leader_board(database, score_getter).await
}

pub async fn cup_leader(cup_number: &str, database: &Database) -> Result<Option<Player>> {
    let score_getter = |player_id| async move {
        database
            .get_player_scores_for_cup_number(player_id, cup_number)
            .await
    };
    Ok(calculate_leader_board(database, score_getter)
        .await?
        .first()
        .map(|x| x.0))
}
