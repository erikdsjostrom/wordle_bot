use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::interaction::application_command::CommandDataOption, utils::MessageBuilder, prelude::RwLock,
};

use crate::{database::CachedDatabase as Database, player::Player, Placement};

// Collects, calculates and presents various statistics
// for a given player.
pub(crate) async fn run(
    player: &Player,
    database: &Arc<RwLock<Database>>,
    _options: &[CommandDataOption],
) -> Result<String> {
    let database = database.read().await;
    let mut response = MessageBuilder::new();
    let gold_medals = database.get_user_gold_medals(player.id as i64).await?;
    let silver_medals = database.get_user_silver_medals(player.id as i64).await?;
    let bronze_medals = database.get_user_bronze_medals(player.id as i64).await?;
    let played_games = database.get_user_played_games(player.id as i64).await?;

    response.push_line(format!("Antal spelade spel: **{}**", played_games));
    for (p, m) in [
        (Placement::Gold.to_string(), gold_medals),
        (Placement::Silver.to_string(), silver_medals),
        (Placement::Bronze.to_string(), bronze_medals),
    ] {
        response.push_line(format!("{p} medaljer: **{m}**"));
    }
    response.push_line("");

    let scores = database.get_user_scores(player.id as i64).await;
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
    response.push_bold_line("Gissningsfördelning:");
    for score in [0, 1, 2, 3, 4, 5, 6] {
        let ratio = 50.0 * score_count[&score] / total;
        response.push_line(format!(
            "{}\t|\t{}|\t{}",
            score,
            "█".repeat(ratio as usize),
            score_count[&score]
        ));
    }

    Ok(response.build())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("stats")
        .description("Lite statistik om en spelare.")
}
