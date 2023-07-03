use anyhow::Result;
use serenity::{
    http::CacheHttp,
    model::prelude::{GuildId, UserId},
};

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct Player {
    pub id: u64,
}

impl From<UserId> for Player {
    fn from(value: UserId) -> Self {
        Player { id: value.0 }
    }
}
impl From<i64> for Player {
    fn from(value: i64) -> Self {
        Player {
            id: value.try_into().unwrap(),
        }
    }
}

impl Player {
    /// Tries to get the nick of the user, if it fails returns the user name
    pub(crate) async fn get_nick(
        &self,
        guild_id: GuildId,
        cache: &impl CacheHttp,
    ) -> Result<String> {
        let user = UserId(self.id).to_user(cache).await?;
        if let Some(nick) = user.nick_in(cache, guild_id).await {
            Ok(nick)
        } else {
            Ok(user.name)
        }
    }
}
