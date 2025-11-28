use reqwest::Client;
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://api.collegefootballdata.com";
const FIRST_WEEK: u8 = 34;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(non_snake_case)]
#[serde(rename_all = "camelCase")]
pub struct GameResult {
    pub id: i32,
    pub season: i32,
    pub week: i32,
    pub season_type: SeasonType,
    pub start_date: String,
    pub start_time_TBD: bool,
    pub completed: bool,
    pub neutral_site: bool,
    pub conference_game: bool,
    pub attendance: Option<i32>,
    pub venue_id: Option<i32>,
    pub venue: Option<String>,
    pub home_id: i32,
    pub home_team: String,
    pub home_conference: Option<String>,
    pub home_classification: Option<Classification>,
    pub home_points: Option<i32>,
    pub home_line_scores: Option<Vec<f64>>,
    pub home_postgame_win_probability: Option<f64>,
    pub home_pregame_elo: Option<i32>,
    pub home_postgame_elo: Option<i32>,
    pub away_id: i32,
    pub away_team: String,
    pub away_conference: Option<String>,
    pub away_classification: Option<Classification>,
    pub away_points: Option<i32>,
    pub away_line_scores: Option<Vec<f64>>,
    pub away_postgame_win_probability: Option<f64>,
    pub away_pregame_elo: Option<i32>,
    pub away_postgame_elo: Option<i32>,
    pub excitement_index: Option<f64>,
    pub highlights: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum SeasonType {
    Regular,
    Postseason,
    Both,
    Allstar,
    SpringRegular,
    SpringPostseason,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Classification {
    Fbs,
    Fcs,
    Ii,
    Iii,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CbbGameResult {
    #[serde(rename = "gameID")]
    pub game_id: u32,
    pub day: String,
    pub home: String,
    pub away: String,
    pub home_score: Option<u32>,
    pub away_score: Option<u32>,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InfoResult {
    pub patron_level: u32,
    pub remaining_calls: u32,
}

pub struct GameResultsApiClient {
    client: Client,
    api_key: String,
}

impl GameResultsApiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub async fn fetch_cfb_game_results(
        &self,
        year: u32,
        week: u8,
    ) -> Result<Vec<GameResult>, reqwest::Error> {
        let week = week - FIRST_WEEK;
        let url = format!("{}/games?year={}&week={}", BASE_URL, year, week);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        let results: Vec<GameResult> = response.json().await?;
        Ok(results)
    }

    pub async fn fetch_cbb_game_results(
        &self,
        day: &str,
    ) -> Result<Vec<CbbGameResult>, reqwest::Error> {
        let url = format!("{}/scoreboard?day={}", BASE_URL, day);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        let results: Vec<CbbGameResult> = response.json().await?;
        Ok(results)
    }

    /// Check API usage/rate limits for College Football Data API
    pub async fn check_usage(&self) -> Result<(), reqwest::Error> {
        // Make a lightweight request to check headers
        let url = format!("{}/info", BASE_URL);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        let result: InfoResult = response.json().await?;
        println!(
            "CFB Data API requests remaining: {}",
            result.remaining_calls
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::game_results_api::GameResultsApiClient;
    use anyhow::Context;
    use chrono::{Datelike, Local};

    #[tokio::test]
    #[ignore]
    async fn test_fetch_games() {
        dotenv::dotenv().ok();
        let api_key = std::env::var("COLLEGE_FOOTBALL_DATA_API_KEY")
            .expect("COLLEGE_FOOTBALL_DATA_API_KEY not set");
        let client = GameResultsApiClient::new(api_key);
        let now = Local::now();
        let year = now.year() as u32;
        let week = now.iso_week().week() as u8;
        println!("Now: {}, year: {}, week: {}", now, year, week);
        let games = client
            .fetch_cfb_game_results(year, week)
            .await
            .context("Failed to fetch CFB game results")
            .unwrap();
        assert!(!games.is_empty());
    }
}
