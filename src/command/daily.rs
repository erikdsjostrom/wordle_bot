use crate::{database::Database, player::Player, Placement};

type NumberOfGuesses = i64;

fn medalists_to_users_and_guesses(
    medalists: Option<Vec<(i64, i64, i64)>>,
) -> Option<(Vec<Player>, NumberOfGuesses)> {
    let medalists = medalists?;
    let score = medalists.first().unwrap().2;
    let user_list: Vec<Player> = medalists
        .into_iter()
        .map(|(id, _, _)| Player::from(id))
        .collect();
    Some((user_list, score))
}

pub async fn fetch_daily_result(
    database: &Database,
) -> [(Placement, Option<(Vec<Player>, NumberOfGuesses)>); 3] {
    [
        (
            Placement::Gold,
            medalists_to_users_and_guesses(database.get_gold_medalist(None).await),
        ),
        (
            Placement::Silver,
            medalists_to_users_and_guesses(database.get_silver_medalist(None).await),
        ),
        (
            Placement::Bronze,
            medalists_to_users_and_guesses(database.get_bronze_medalist(None).await),
        ),
    ]
}
