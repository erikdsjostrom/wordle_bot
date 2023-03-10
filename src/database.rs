use std::future::Future;

use log::debug;

use crate::command::score::FIB;
use crate::error::Result;
use crate::player::Player;
use crate::scoresheet::Scoresheet;
use crate::utils::{self, current_cup_number};

pub(crate) struct Database {
    database: sqlx::SqlitePool,
}

impl Database {
    pub async fn new(filename: &str) -> Result<Self> {
        // Initiate a connection to the database file, creating the file if required,
        let database = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(filename)
                    .create_if_missing(true),
            )
            .await?;
        // Run migrations, which updates the database's schema to the latest version.
        sqlx::migrate!("./migrations").run(&database).await?;
        Ok(Self { database })
    }

    pub async fn get_daily_day(&self) -> Result<i64> {
        sqlx::query!("SELECT max(id) as id from daily")
            .fetch_one(&self.database)
            .await
            .map(|row| row.id.into())
            .map_err(|err| err.into())
    }

    pub async fn get_gold_medalist(&self, day: Option<i64>) -> Result<Option<Vec<Scoresheet>>> {
        let day = match day {
            Some(day) => day,
            None => self.get_daily_day().await?,
        };
        let medalist: Vec<Scoresheet> = sqlx::query_as!(Scoresheet, "SELECT * from score_sheet WHERE day = ? AND score = (SELECT gold from daily where id = ?)", day, day)
            .fetch_all(&self.database)
            .await?;
        Ok(match medalist.is_empty() {
            true => None,
            false => Some(medalist),
        })
    }

    pub async fn get_silver_medalist(&self, day: Option<i64>) -> Result<Option<Vec<Scoresheet>>> {
        let day = match day {
            Some(day) => day,
            None => self.get_daily_day().await?,
        };
        let medalist: Vec<_> = sqlx::query_as!(Scoresheet, "SELECT * from score_sheet WHERE day = ? AND score = (SELECT silver from daily where id = ?)", day, day)
        .fetch_all(&self.database)
        .await?;
        Ok(match medalist.is_empty() {
            true => None,
            false => Some(medalist),
        })
    }
    pub async fn get_bronze_medalist(&self, day: Option<i64>) -> Result<Option<Vec<Scoresheet>>> {
        let day = match day {
            Some(day) => day,
            None => self.get_daily_day().await?,
        };
        let medalist: Vec<_> = sqlx::query_as!(Scoresheet, "SELECT * from score_sheet WHERE day = ? AND score = (SELECT bronze from daily where id = ?)", day, day)
        .fetch_all(&self.database)
        .await?;
        Ok(match medalist.is_empty() {
            true => None,
            false => Some(medalist),
        })
    }

    pub async fn get_daily_high_scores(&self, day: i64) -> Result<[Option<i64>; 3]> {
        let scores = sqlx::query!("SELECT gold, silver, bronze FROM daily WHERE id = ?", day)
            .fetch_one(&self.database)
            .await?;

        Ok([scores.gold, scores.silver, scores.bronze])
    }

    pub async fn get_user_played_games(&self, user_id: i64) -> Result<i32> {
        let res = sqlx::query!(
            "SELECT COUNT(id) as count FROM score_sheet WHERE player_id = ?",
            user_id
        )
        .fetch_one(&self.database)
        .await?
        .count;
        Ok(res)
    }

    pub async fn get_user_gold_medals(&self, user_id: i64) -> Result<i32> {
        let res = sqlx::query!("SELECT COUNT(score) as amount FROM score_sheet JOIN daily ON score_sheet.score = daily.gold AND score_sheet.day = daily.id AND score_sheet.player_id = ?", user_id).fetch_one(&self.database).await?.amount;
        Ok(res)
    }

    pub async fn get_user_silver_medals(&self, user_id: i64) -> Result<i32> {
        let res = sqlx::query!("SELECT COUNT(score) as amount FROM score_sheet JOIN daily ON score_sheet.score = daily.silver AND score_sheet.day = daily.id AND score_sheet.player_id = ?", user_id).fetch_one(&self.database).await?.amount;
        Ok(res)
    }

    pub async fn get_user_bronze_medals(&self, user_id: i64) -> Result<i32> {
        let res = sqlx::query!("SELECT COUNT(score) as amount FROM score_sheet JOIN daily ON score_sheet.score = daily.bronze AND score_sheet.day = daily.id AND score_sheet.player_id = ?", user_id).fetch_one(&self.database).await?.amount;
        Ok(res)
    }

    pub async fn get_user_scores(&self, user_id: i64) -> Vec<i64> {
        // Grab all wordle scores
        sqlx::query!("SELECT score FROM score_sheet WHERE player_id = ?", user_id)
            .fetch_all(&self.database)
            .await
            .unwrap_or_default()
            .iter()
            .map(|r| r.score)
            .collect()
    }

    pub async fn new_player(&self, player_id: i64) -> Result<()> {
        sqlx::query!(
            "INSERT INTO player (id) VALUES (?) ON CONFLICT DO NOTHING",
            player_id
        )
        .execute(&self.database)
        .await?;
        Ok(())
    }

    pub async fn new_daily(&self, day: i64) -> Result<()> {
        sqlx::query!(
            "INSERT INTO daily (id) VALUES (?) ON CONFLICT DO NOTHING",
            day
        )
        .execute(&self.database)
        .await?;
        Ok(())
    }

    pub async fn new_score_sheet(
        &self,
        msg_id: i64,
        day: i64,
        player_id: i64,
        score: i64,
        cup_number: String,
    ) -> Result<()> {
        // Conflict = Cheater
        sqlx::query!(
        "INSERT INTO score_sheet (msg_id, day, player_id, score, cup_number) VALUES (?, ?, ?, ?, ?) ON CONFLICT DO NOTHING",
        msg_id,
        day,
        player_id,
        score,
        cup_number,
    )
    .execute(&self.database)
    .await?;
        Ok(())
    }

    pub async fn update_daily(
        &self,
        day: i64,
        gold: Option<i64>,
        silver: Option<i64>,
        bronze: Option<i64>,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE daily SET (gold, silver, bronze) = (?, ?, ?) WHERE id = ?",
            gold,
            silver,
            bronze,
            day
        )
        .execute(&self.database)
        .await?;
        Ok(())
    }

    pub async fn get_players(&self) -> Result<Vec<i64>> {
        let res = sqlx::query!("SELECT id FROM player")
            .fetch_all(&self.database)
            .await?
            .iter()
            .map(|player| player.id)
            .collect();
        Ok(res)
    }

    pub async fn get_player_scores_from_day(
        &self,
        from_day: i64,
        player_id: i64,
    ) -> Result<Vec<i64>> {
        sqlx::query!(
            "SELECT score FROM score_sheet WHERE player_id = ? AND day >= ?",
            player_id,
            from_day
        )
        .fetch_all(&self.database)
        .await
        .map(|row| row.iter().map(|score_sheet| score_sheet.score).collect())
        .map_err(|err| err.into())
    }

    pub async fn get_player_scores_for_current_cup(&self, player_id: i64) -> Result<Vec<i64>> {
        let cup_number = current_cup_number();
        self.get_player_scores_for_cup_number(player_id, &cup_number)
            .await
    }

    pub async fn get_player_scores_for_cup_number(
        &self,
        player_id: i64,
        cup_number: &str,
    ) -> Result<Vec<i64>> {
        debug!("Fetching score sheets for cup number {cup_number}");
        sqlx::query!(
            "SELECT score FROM score_sheet WHERE player_id = ? AND cup_number = ?",
            player_id,
            cup_number
        )
        .fetch_all(&self.database)
        .await
        .map(|row| row.iter().map(|score_sheet| score_sheet.score).collect())
        .map_err(|err| err.into())
    }

    // Calculate the current score of all players
    // The wordle scores are used as indexes into the fibonachi sequence
    // to lend more weight to earlier correct guesses
    // TODO: Factor out database to make it testable
    async fn calculate_leader_board<F, Fut>(&self, score_getter: F) -> Result<Vec<(Player, u32)>>
    where
        F: Fn(i64) -> Fut,
        Fut: Future<Output = Result<Vec<i64>>>,
    {
        debug!("Calculating leader board");
        let mut fetched: usize = 0;
        let mut leader_board: Vec<(Player, u32)> = vec![];
        let players = self.get_players().await?;
        for player_id in players {
            let scores = score_getter(player_id).await?;
            fetched += scores.len();
            let score = scores.iter().fold(0, |acc, elem| acc + FIB[*elem as usize]);
            if score == 0 {
                continue;
            }
            leader_board.push((Player::from(player_id), score));
        }
        debug!("Fetched {fetched} score sheets.");
        // XXX
        leader_board.sort_by_key(|x| x.1);
        leader_board.reverse();
        Ok(leader_board)
    }

    pub async fn total_cup_score(&self) -> Result<Vec<(Player, u32)>> {
        let score_getter =
            |player_id| async move { self.get_player_scores_from_day(0, player_id).await };
        self.calculate_leader_board(score_getter).await
    }

    pub async fn current_cup_score(&self) -> Result<Vec<(Player, u32)>> {
        let score_getter =
            |player_id| async move { self.get_player_scores_for_current_cup(player_id).await };
        self.calculate_leader_board(score_getter).await
    }

    pub async fn cup_leader(&self, cup_number: &str) -> Result<Option<Player>> {
        let score_getter = |player_id| async move {
            self.get_player_scores_for_cup_number(player_id, cup_number)
                .await
        };
        Ok(self
            .calculate_leader_board(score_getter)
            .await?
            .first()
            .map(|x| x.0))
    }

    pub async fn current_cup_leader(&self) -> Result<Option<Player>> {
        let current_cup = utils::current_cup_number();
        self.cup_leader(&current_cup).await
    }
}
