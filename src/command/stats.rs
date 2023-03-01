use std::{collections::HashMap, fmt::Write};

use crate::{database::Database, Placement};

// Collects, calculates and presents various statistics
// for a given player.
pub async fn stats(user_id: i64, database: &Database) -> String {
    let mut response: String = String::from("");
    let gold_medals = database.get_user_gold_medals(user_id).await;
    let silver_medals = database.get_user_silver_medals(user_id).await;
    let bronze_medals = database.get_user_bronze_medals(user_id).await;
    let played_games = database.get_user_played_games(user_id).await;

    writeln!(response, "Antal spelade spel: **{}**", played_games).unwrap();
    for (p, m) in [
        (Placement::Gold.to_string(), gold_medals),
        (Placement::Silver.to_string(), silver_medals),
        (Placement::Bronze.to_string(), bronze_medals),
    ] {
        writeln!(response, "{p} medaljer: **{m}**",).unwrap();
    }
    writeln!(response, "").unwrap();

    let scores = database.get_user_scores(user_id).await;
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
    for score in scores {
        *score_count.get_mut(&score).unwrap() += 1.0;
    }
    writeln!(response, "Poängfördelning:").unwrap();
    for score in [0, 1, 2, 3, 4, 5, 6] {
        let ratio = 50.0 * score_count[&score] / total;
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
