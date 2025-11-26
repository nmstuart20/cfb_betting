use serde::{Deserialize, Serialize};
use reqwest::Client;

const BASE_URL: &str = "https://api.collegefootballdata.com";

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameResult {
    pub id: u32,
    pub season: u32,
    pub week: u8,
    pub season_type: String,
    pub start_date: String,
    pub home_team: String,
    pub home_conference: Option<String>,
    pub home_points: Option<u32>,
    pub away_team: String,
    pub away_conference: Option<String>,
    pub away_points: Option<u32>,
    pub completed: bool,
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

    pub async fn fetch_cfb_game_results(&self, year: u32, week: u8, season_type: &str, conference: &str) -> Result<Vec<GameResult>, reqwest::Error> {
        let url = format!(
            "{}/games?year={}&week={}&seasonType={}&conference={}",
            BASE_URL, year, week, season_type, conference
        );

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        let results: Vec<GameResult> = response.json().await?;
        Ok(results)
    }

    pub async fn fetch_cbb_game_results(&self, day: &str) -> Result<Vec<CbbGameResult>, reqwest::Error> {
        let url = format!(
            "{}/scoreboard?day={}",
            BASE_URL, day
        );

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        let results: Vec<CbbGameResult> = response.json().await?;
        Ok(results)
    }
}
