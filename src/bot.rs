use std::sync::Arc;

use anyhow::Result;
use log::{debug, error, info};
use serenity::{
    futures::StreamExt,
    model::prelude::{ChannelId, Message, ReactionType},
    prelude::{Context, RwLock},
};

use crate::{
    database::CachedDatabase as Database,
    parser,
    utils::{cup_number_from_unixtime, recalcualate_high_scores},
    Placement,
};

pub(crate) struct Bot {
    pub database: Arc<RwLock<Database>>,
}

impl Bot {
    pub(crate) async fn set_medals(
        &self,
        day: i64,
        channel_id: ChannelId,
        http: &Context,
    ) -> Result<()> {
        // playerId, msgId, score
        let database = self.database.read().await;
        for (p, medalists) in [
            (
                Placement::Gold,
                database.get_gold_medalist(Some(day)).await?,
            ),
            (
                Placement::Silver,
                database.get_silver_medalist(Some(day)).await?,
            ),
            (
                Placement::Bronze,
                database.get_bronze_medalist(Some(day)).await?,
            ),
        ] {
            let Some(medalists) = medalists else {continue;};
            for medalist in medalists {
                channel_id
                    .create_reaction(
                        http,
                        medalist.message_id(),
                        ReactionType::Unicode(p.to_string()),
                    )
                    .await?;
            }
        }
        Ok(())
    }

    async fn clear_medals(&self, day: i64, channel_id: ChannelId, http: &Context) -> Result<()> {
        let database = self.database.read().await;
        for (p, medalists) in [
            (
                Placement::Gold,
                database.get_gold_medalist(Some(day)).await?,
            ),
            (
                Placement::Silver,
                database.get_silver_medalist(Some(day)).await?,
            ),
            (
                Placement::Bronze,
                database.get_bronze_medalist(Some(day)).await?,
            ),
        ] {
            let Some(medalists) = medalists else {continue;};
            for medalist in medalists {
                channel_id
                    .delete_reaction(
                        http,
                        medalist.message_id(),
                        None,
                        ReactionType::Unicode(p.to_string()),
                    )
                    .await?;
            }
        }
        Ok(())
    }
    async fn new_score_sheet(&self, msg: &Message) -> Result<()> {
        let cup_number = cup_number_from_unixtime(msg.timestamp.unix_timestamp());
        let player_id = msg.author.id.0 as i64;
        let msg_id = msg.id.0 as i64;
        let (day, score) = parser::parse_msg(&msg.content)?;
        let database = self.database.write().await;
        // Create new player if not exists
        database.new_player(player_id).await?;
        // TODO: Is there a better place to do this to avoid runtime error if this is not executed first?
        database.new_daily(day).await?;
        debug!("Day: {}, Score: {}, Cup number: {}", day, score, cup_number);
        self.new_daily_score(Some(day), score).await?;
        database
            .new_score_sheet(msg_id, day, player_id, score, cup_number)
            .await?;
        Ok(())
    }

    pub(crate) async fn read_old_messages(&self, channel_id: ChannelId, http: &Context) {
        debug!("Reading old messages");
        let mut msg_count: i64 = 0;
        let mut messages = channel_id.messages_iter(http).boxed();
        while let Some(msg_result) = messages.next().await {
            let msg = match msg_result {
                Err(e) => {
                    error!("{e}");
                    return;
                }
                Ok(msg) => msg,
            };
            if msg.content.starts_with("Wordle") {
                msg_count += 1;
                _ = self.new_score_sheet(&msg).await;
            }
        }
        info!("Old messages read: {msg_count}");
    }

    async fn new_daily_score(&self, day: Option<i64>, score: i64) -> Result<()> {
        if score == 0 {
            return Ok(());
        }
        let database = self.database.write().await;
        let day = day.unwrap_or(database.get_daily_day().await?);
        let high_scores = database.get_daily_high_scores(day).await?;
        let high_scores = recalcualate_high_scores(high_scores, score);
        database
            .update_daily(day, high_scores[0], high_scores[1], high_scores[2])
            .await
    }

    pub(crate) async fn handle_wordle_message(
        &self,
        msg: &Message,
        ctx: &Context,
    ) -> Result<()> {
        let (day, _) = parser::parse_msg(&msg.content)?;
        self.clear_medals(day, msg.channel_id, ctx).await?;
        self.new_score_sheet(msg).await?;
        self.set_medals(day, msg.channel_id, ctx).await?;
        self.database.write().await.update_cache().await?;
        Ok(())
    }
}
