use anyhow::Result;
use serenity::{
    builder::CreateApplicationCommand,
    model::prelude::interaction::application_command::CommandDataOption, prelude::Context,
    utils::MessageBuilder,
};

use super::score::FIB;
use crate::{database::Database, scoresheet::Scoresheet, Placement, GUILD_ID};

pub(crate) async fn run(
    database: &Database,
    ctx: &Context,
    _options: &[CommandDataOption],
) -> Result<String> {
    let daily = fetch_daily_result(database).await?;
    let mut response = MessageBuilder::new();
    response.push_bold_line("Dagens placering:");
    for (placement, score_sheets) in daily {
        let Some(score_sheets) = score_sheets else {break;};
        let score = score_sheets[0].score();
        let mut users: Vec<String> = Vec::default();
        for sheet in score_sheets {
            let nick = sheet
                .player()
                .get_nick(GUILD_ID.into(), &ctx.http)
                .await
                .unwrap();
            users.push(nick);
        }
        let line = format!(
            "{} - {} - {} försök ({}p)",
            placement,
            users.join(", "),
            score,
            FIB[score as usize]
        );
        response.push_line(line);
    }
    Ok(response.build())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("dagens").description("Dagens gissningar.")
}

async fn fetch_daily_result(
    database: &Database,
) -> Result<[(Placement, Option<Vec<Scoresheet>>); 3]> {
    Ok([
        (Placement::Gold, database.get_gold_medalist(None).await?),
        (Placement::Silver, database.get_silver_medalist(None).await?),
        (Placement::Bronze, database.get_bronze_medalist(None).await?),
    ])
}
