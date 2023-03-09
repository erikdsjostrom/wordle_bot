use log::debug;

use crate::error::Result;
use crate::utils::current_cup_number;

pub struct Database {
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

    async fn get_scores(&self, day: Option<i64>) -> Result<Vec<(i64, (i64, i64))>> {
        let res = if let Some(day) = day {
            sqlx::query!(
            "SELECT player_id, msg_id, score FROM score_sheet where day = ? ORDER BY score DESC",
            day
        )
            .fetch_all(&self.database)
            .await?
            .iter()
            .map(|x| (x.score, (x.msg_id, x.player_id)))
            .collect()
        } else {
            sqlx::query!("SELECT msg_id, player_id, score FROM score_sheet where day = (SELECT max(day) from score_sheet) ORDER BY score DESC")
    .fetch_all(&self.database)
    .await?
        .iter()
        .map(|x| (x.score, (x.msg_id, x.player_id)))
        .collect()
        };
        Ok(res)
    }

    pub async fn get_daily_day(&self) -> Result<i64> {
        sqlx::query!("SELECT max(id) as id from daily")
            .fetch_one(&self.database)
            .await
            .map(|row| row.id.into())
            .map_err(|err| err.into())
    }

    pub async fn get_silver_medalist(
        &self,
        day: Option<i64>,
    ) -> Result<Option<Vec<(i64, i64, i64)>>> {
        let day = match day {
            Some(day) => day,
            None => self.get_daily_day().await?,
        };
        let medalist: Vec<_> = sqlx::query!("SELECT * from score_sheet WHERE day = ? AND score = (SELECT silver from daily where id = ?)", day, day)
        .fetch_all(&self.database)
        .await?;
        let medalist: Vec<_> = medalist
            .iter()
            .map(|sheet| (sheet.player_id, sheet.msg_id, sheet.score))
            .collect();
        Ok(match medalist.is_empty() {
            true => None,
            false => Some(medalist),
        })
    }
    pub async fn get_bronze_medalist(
        &self,
        day: Option<i64>,
    ) -> Result<Option<Vec<(i64, i64, i64)>>> {
        let day = match day {
            Some(day) => day,
            None => self.get_daily_day().await?,
        };
        let medalist: Vec<_> = sqlx::query!("SELECT * from score_sheet WHERE day = ? AND score = (SELECT bronze from daily where id = ?)", day, day)
        .fetch_all(&self.database)
        .await?;
        let medalist: Vec<_> = medalist
            .iter()
            .map(|sheet| (sheet.player_id, sheet.msg_id, sheet.score))
            .collect();
        Ok(match medalist.is_empty() {
            true => None,
            false => Some(medalist),
        })
    }
    pub async fn get_gold_medalist(
        &self,
        day: Option<i64>,
    ) -> Result<Option<Vec<(i64, i64, i64)>>> {
        let day = match day {
            Some(day) => day,
            None => self.get_daily_day().await?,
        };
        let medalist: Vec<_> = sqlx::query!("SELECT * from score_sheet WHERE day = ? AND score = (SELECT gold from daily where id = ?)", day, day)
            .fetch_all(&self.database)
            .await?;
        let medalist: Vec<_> = medalist
            .iter()
            .map(|sheet| (sheet.player_id, sheet.msg_id, sheet.score))
            .collect();
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
}
