use std::future::Future;

use log::debug;

use crate::{database::Database, utils::current_cup_number, player::Player};

const DAY_OF_INCEPTION: i64 = 580;
// Failure (X) gives a score of zero
pub const FIB: [u32; 7] = [0, 13, 8, 5, 3, 2, 1];

// Calculate the current score of all players
// The wordle scores are used as indexes into the fibonachi sequence
// to lend more weight to earlier correct guesses
// TODO: Factor out database to make it testable
async fn calculate_leader_board<F, Fut>(database: &Database, score_getter: F) -> Vec<(Player, u32)>
where
    F: Fn(i64) -> Fut,
    Fut: Future<Output = Vec<i64>>,
{
    debug!("Calculating leader board");
    let mut fetched: usize = 0;
    let mut leader_board: Vec<(Player, u32)> = vec![];
    let players = database.get_players().await;
    for player_id in players {
        let scores = score_getter(player_id).await;
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
    leader_board
}

pub async fn score_since_inception(database: &Database) -> Vec<(Player, u32)> {
    let score_getter = |player_id| async move {
        database
            .get_player_scores_from_day(DAY_OF_INCEPTION, player_id)
            .await
    };
    calculate_leader_board(database, score_getter).await
}

pub async fn total_cup_score(database: &Database) -> Vec<(Player, u32)> {
    let score_getter =
        |player_id| async move { database.get_player_scores_from_day(0, player_id).await };
    calculate_leader_board(database, score_getter).await
}

pub async fn current_cup_score(database: &Database) -> Vec<(Player, u32)> {
    let score_getter =
        |player_id| async move { database.get_player_scores_for_current_cup(player_id).await };
    calculate_leader_board(database, score_getter).await
}

pub async fn cup_leader(cup_number: &str, database: &Database) -> Option<Player> {
    let score_getter = |player_id| async move {
        database
            .get_player_scores_for_cup_number(player_id, cup_number)
            .await
    };
    Some(calculate_leader_board(database, score_getter).await.first()?.0)
}

pub async fn current_cup_leader(database: &Database) -> Option<Player> {
    let cup_number = current_cup_number();
    cup_leader(&cup_number, database).await
}
