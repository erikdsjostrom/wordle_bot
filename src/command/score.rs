use serenity::model::prelude::interaction::application_command::{
    CommandDataOption, CommandDataOptionValue,
};
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use serenity::{builder::CreateApplicationCommand, model::prelude::command::CommandOptionType};

use crate::{database::Database, error::Result, utils::current_cup_number_cute_format, GUILD_ID};

// Failure (X) gives a score of zero
pub const FIB: [u32; 7] = [0, 13, 8, 5, 3, 2, 1];

pub(crate) async fn run(
    database: &Database,
    ctx: &Context,
    options: &[CommandDataOption],
) -> Result<String> {
    let totala: bool = {
        if let Some(opt) = options.get(0) {
            if let Some(opt) = opt.resolved.as_ref() {
                if let CommandDataOptionValue::Boolean(total) = opt {
                    *total
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    };
    let mut response = MessageBuilder::new();
    let score = if totala {
        response.push_bold_line(format!("Ställning i totalcupen:"));
        database.total_cup_score().await?
    } else {
        let cup_number = current_cup_number_cute_format();
        response.push_bold_line(format!("Ställning i månadscupen {cup_number}:"));
        database.current_cup_score().await?
    };
    for (player, result) in score {
        let user = player.get_nick(GUILD_ID.into(), &ctx.http).await?;
        response.push_line(format!("\t{user}: {result}"));
    }
    Ok(response.build())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("ställning")
        .description("Nuvarande ställning i månadscupen.")
        .create_option(|option| {
            option
                .name("Totala")
                .description("")
                .kind(CommandOptionType::Boolean)
                .required(false)
        })
}
