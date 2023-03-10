use serenity::model::prelude::MessageId;

use crate::player::Player;

#[allow(dead_code)]
#[derive(sqlx::FromRow)]
pub(crate) struct Scoresheet {
    pub(crate) id: i64,
    pub(crate) msg_id: i64,
    pub(crate) day: i64,
    pub(crate) player_id: i64,
    pub(crate) score: i64,
    pub(crate) cup_number: String,
}

impl Scoresheet {
    pub(crate) fn player(&self) -> Player {
        Player::from(self.player_id)
    }

    pub(crate) fn message_id(&self) -> MessageId {
        MessageId(self.msg_id as u64)
    }

    pub(crate) fn score(&self) -> i64 {
        self.score
    }
}
