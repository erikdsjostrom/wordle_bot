use std::fmt::Write as _;
use serenity::{prelude::Context, model::prelude::interaction::application_command::CommandDataOption, builder::CreateApplicationCommand};

use crate::{error::Result, database::Database, player::Player, Placement, GUILD_ID};

use super::score::FIB;

type NumberOfGuesses = i64;

pub async fn run(database: &Database, ctx: &Context, _options: &[CommandDataOption]) -> Result<String> {
    let daily = fetch_daily_result(database).await?;
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
        writeln!(
            response,
            "{} - {} - {} försök ({}p)",
            placement.to_string(),
            users,
            medalists.1,
            FIB[medalists.1 as usize]
        )?;
    }
    Ok(response)
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("dagens")
        .description("Dagens gissningar.")
}

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

async fn fetch_daily_result(
    database: &Database,
) -> Result<[(Placement, Option<(Vec<Player>, NumberOfGuesses)>); 3]> {
    Ok([
        (
            Placement::Gold,
            medalists_to_users_and_guesses(database.get_gold_medalist(None).await?),
        ),
        (
            Placement::Silver,
            medalists_to_users_and_guesses(database.get_silver_medalist(None).await?),
        ),
        (
            Placement::Bronze,
            medalists_to_users_and_guesses(database.get_bronze_medalist(None).await?),
        ),
    ])
}
