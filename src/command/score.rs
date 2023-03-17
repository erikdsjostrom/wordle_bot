use log::debug;
use serenity::model::prelude::command::CommandOptionType;
use serenity::{builder::CreateApplicationCommand, model::prelude::interaction::application_command::CommandDataOptionValue};
use serenity::model::prelude::interaction::application_command::CommandDataOption;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;

use crate::{database::Database, error::Result, utils::current_cup_number_cute_format, GUILD_ID};

// Failure (X) gives a score of zero
pub const FIB: [u32; 7] = [0, 13, 8, 5, 3, 2, 1];

pub(crate) async fn run(
    database: &Database,
    ctx: &Context,
    options: &[CommandDataOption],
) -> Result<String> {
    debug!("{:?}", options);
    // XXX
    let totala: bool = {
        if let Some(opt) = options.get(0) {
            if let Some(CommandDataOptionValue::Boolean(total)) = opt.resolved.as_ref() {
                    *total
            } else {
                false
            }
        } else {
            false
        }
    };
    debug!("Totala: {totala}");
    let mut response = MessageBuilder::new();
    let score = if totala {
        response.push_bold_line("Ställning i totalcupen:");
        database.total().await?.iter().map(|(k, v)| (*k,*v)).collect()
    } else {
        let cup_number = current_cup_number_cute_format();
        response.push_bold_line(format!("Ställning i månadscupen {cup_number}:"));
        database.current_cup_score().await?
    };
    debug!("Writing results.");
    for (player, result) in score {
        let user = player.get_nick(GUILD_ID.into(), &ctx.http).await?;
        response.push_line(format!("\t\t{result}: {user}"));
    }
    let response = response.build();
    dbg!("Sending response");
    Ok(response)
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("ställning")
        .description("Nuvarande ställning i månadscupen.")
    .create_option(|option| {
        option
            .name("totala")
            .description("ställning i totalcupen.")
            .kind(CommandOptionType::Boolean)
            .required(false)
    })
}
