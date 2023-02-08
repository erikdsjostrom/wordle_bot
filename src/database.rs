#![allow(dead_code)] // TODO
                     // Setters and getters to the database
                     // use sqlx::{QueryBuilder, Sqlite};

// type Database = sqlx::Pool<sqlx::Sqlite>;
use log::error;

pub struct Database {
    database: sqlx::SqlitePool,
}

impl Database {
    pub async fn new(filename: &str) -> Self {
        // Initiate a connection to the database file, creating the file if required,
        let database = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(filename)
                    .create_if_missing(true),
            )
            .await
            .expect("Couldn't connect to database");
        // Run migrations, which updates the database's schema to the latest version.
        sqlx::migrate!("./migrations")
            .run(&database)
            .await
            .expect("Couldn't run database migrations");
        Self { database }
    }

    async fn get_scores(&self, day: Option<i64>) -> Vec<(i64, (i64, i64))> {
        if let Some(day) = day {
            sqlx::query!(
            "SELECT player_id, msg_id, score FROM score_sheet where day = ? ORDER BY score DESC",
            day
        )
            .fetch_all(&self.database)
            .await
            .unwrap()
            .iter()
            .map(|x| (x.score, (x.msg_id, x.player_id)))
            .collect()
        } else {
            sqlx::query!("SELECT msg_id, player_id, score FROM score_sheet where day = (SELECT max(day) from score_sheet) ORDER BY score DESC")
    .fetch_all(&self.database)
    .await
        .unwrap()
        .iter()
        .map(|x| (x.score, (x.msg_id, x.player_id)))
        .collect()
        }
    }

    pub async fn get_daily_day(&self) -> i64 {
        sqlx::query!("SELECT max(id) as id from daily")
            .fetch_one(&self.database)
            .await
            .unwrap()
            .id
            .into()
    }

    pub async fn get_silver_medalist(&self, day: Option<i64>) -> Option<Vec<(i64, i64, i64)>> {
        let day = if day.is_none() {
            self.get_daily_day().await
        } else {
            day.unwrap()
        };
        let medalist: Vec<_> = sqlx::query!("SELECT * from score_sheet WHERE day = ? AND score = (SELECT silver from daily where id = ?)", day, day)
        .fetch_all(&self.database)
        .await
        .unwrap();
        dbg!(&medalist);
        let medalist: Vec<_> = medalist
            .iter()
            .map(|sheet| (sheet.player_id, sheet.msg_id, sheet.score))
            .collect();
        dbg!(&medalist);
        if medalist.len() == 0 {
            None
        } else {
            Some(medalist)
        }
    }
    pub async fn get_bronze_medalist(&self, day: Option<i64>) -> Option<Vec<(i64, i64, i64)>> {
        let day = if day.is_none() {
            self.get_daily_day().await
        } else {
            day.unwrap()
        };
        let medalist: Vec<_> = sqlx::query!("SELECT * from score_sheet WHERE day = ? AND score = (SELECT bronze from daily where id = ?)", day, day)
        .fetch_all(&self.database)
        .await
        .unwrap();
        dbg!(&medalist);
        let medalist: Vec<_> = medalist
            .iter()
            .map(|sheet| (sheet.player_id, sheet.msg_id, sheet.score))
            .collect();
        dbg!(&medalist);
        if medalist.len() == 0 {
            None
        } else {
            Some(medalist)
        }
    }
    pub async fn get_gold_medalist(&self, day: Option<i64>) -> Option<Vec<(i64, i64, i64)>> {
        let day = if day.is_none() {
            self.get_daily_day().await
        } else {
            day.unwrap()
        };
        let medalist: Vec<_> = sqlx::query!("SELECT * from score_sheet WHERE day = ? AND score = (SELECT gold from daily where id = ?)", day, day)
            .fetch_all(&self.database)
            .await
            .unwrap();
        dbg!(&medalist);
        let medalist: Vec<_> = medalist
            .iter()
            .map(|sheet| (sheet.player_id, sheet.msg_id, sheet.score))
            .collect();
        dbg!(&medalist);
        if medalist.len() == 0 {
            None
        } else {
            Some(medalist)
        }
    }

    pub async fn get_daily_high_scores(&self, day: i64) -> [Option<i64>; 3] {
        let scores = match sqlx::query!("SELECT gold, silver, bronze FROM daily WHERE id = ?", day)
            .fetch_one(&self.database)
            .await
        {
            Ok(x) => x,
            Err(e) => {
                error!("{e}");
                panic!();
            }
        };

        [scores.gold, scores.silver, scores.bronze]
    }

    pub async fn get_user_played_games(&self, user_id: i64) -> i32 {
        sqlx::query!(
            "SELECT COUNT(id) as count FROM score_sheet WHERE player_id = ?",
            user_id
        )
        .fetch_one(&self.database)
        .await
        .unwrap()
        .count
    }

    pub async fn get_user_gold_medals(&self, user_id: i64) -> i32 {
        sqlx::query!("SELECT COUNT(score) as amount FROM score_sheet JOIN daily ON score_sheet.score = daily.gold AND score_sheet.day = daily.id AND score_sheet.player_id = ?", user_id).fetch_one(&self.database).await.unwrap().amount
    }

    pub async fn get_user_silver_medals(&self, user_id: i64) -> i32 {
        sqlx::query!("SELECT COUNT(score) as amount FROM score_sheet JOIN daily ON score_sheet.score = daily.silver AND score_sheet.day = daily.id AND score_sheet.player_id = ?", user_id).fetch_one(&self.database).await.unwrap().amount
    }

    pub async fn get_user_bronze_medals(&self, user_id: i64) -> i32 {
        sqlx::query!("SELECT COUNT(score) as amount FROM score_sheet JOIN daily ON score_sheet.score = daily.bronze AND score_sheet.day = daily.id AND score_sheet.player_id = ?", user_id).fetch_one(&self.database).await.unwrap().amount
    }

    pub async fn get_user_scores(&self, user_id: i64) -> Vec<i64> {
        // Gran all wordle scores
        sqlx::query!("SELECT score FROM score_sheet WHERE player_id = ?", user_id)
            .fetch_all(&self.database)
            .await
            .unwrap_or_default()
            .iter()
            .map(|r| r.score)
            .collect()
    }

    pub async fn new_player(&self, player_id: i64) {
        sqlx::query!(
            "INSERT INTO player (id) VALUES (?) ON CONFLICT DO NOTHING",
            player_id
        )
        .execute(&self.database)
        .await
        .unwrap();
    }

    pub async fn new_daily(&self, day: i64) {
        sqlx::query!(
            "INSERT INTO daily (id) VALUES (?) ON CONFLICT DO NOTHING",
            day
        )
        .execute(&self.database)
        .await
        .unwrap();
    }

    pub async fn new_score_sheet(&self, msg_id: i64, day: i64, player_id: i64, score: i64) {
        // Conflict = Cheater
        sqlx::query!(
        "INSERT INTO score_sheet (msg_id, day, player_id, score) VALUES (?, ?, ?, ?) ON CONFLICT DO NOTHING",
        msg_id,
        day,
        player_id,
        score,
    )
    .execute(&self.database) // < Where the command will be executed
    .await
    .unwrap();
    }

    pub async fn update_daily(
        &self,
        day: i64,
        gold: Option<i64>,
        silver: Option<i64>,
        bronze: Option<i64>,
    ) {
        sqlx::query!(
            "UPDATE daily SET (gold, silver, bronze) = (?, ?, ?) WHERE id = ?",
            gold,
            silver,
            bronze,
            day
        )
        .execute(&self.database)
        .await
        .unwrap();
    }

    pub async fn get_players(&self) -> Vec<i64> {
        sqlx::query!("SELECT id FROM player")
            .fetch_all(&self.database)
            .await
            .unwrap()
            .iter()
            .map(|player| player.id)
            .collect()
    }

    pub async fn get_player_scores_from_day(&self, from_day: i64, player_id: i64) -> Vec<i64> {
        sqlx::query!(
            "SELECT score FROM score_sheet WHERE player_id = ? AND day >= ?",
            player_id,
            from_day
        )
        .fetch_all(&self.database)
        .await
        .unwrap()
        .iter()
        .map(|score_sheet| score_sheet.score)
        .collect()
    }
}
