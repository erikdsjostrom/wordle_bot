use log::error;
use serenity::{
    http::CacheHttp,
    model::prelude::{GuildId, UserId},
};

#[derive(Copy, Clone, Debug)]
pub struct Player {
    player_id: u64,
}

impl From<i64> for Player {
    fn from(value: i64) -> Self {
        Player {
            player_id: value.try_into().unwrap(),
        }
    }
}

impl Player {

    /// Tries to get the nick of the user, if it fails returns the user name
    pub(crate) async fn get_nick(&self, guild_id: GuildId, cache: &impl CacheHttp) -> Option<String> {
        let user = match UserId(self.player_id).to_user(cache).await {
            Ok(user) => user,
            Err(e) => {
                error!("Error retriving user: {e}");
                return None;
            }
        };
        user.nick_in(cache, guild_id).await.or(Some(user.name))
    }
}
