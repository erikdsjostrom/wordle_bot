use std::fmt::Write as _;

use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::interaction::application_command::CommandDataOption;
use serenity::prelude::*;

use crate::{
    database::Database, error::Result, utils::current_cup_number_cute_format,
    GUILD_ID,
};

// Failure (X) gives a score of zero
pub const FIB: [u32; 7] = [0, 13, 8, 5, 3, 2, 1];

pub(crate) async fn run(
    database: &Database,
    ctx: &Context,
    _options: &[CommandDataOption],
) -> Result<String> {
    let score = database.current_cup_score().await?;
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
