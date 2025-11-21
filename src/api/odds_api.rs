use crate::models::{BettingOdds, Game, MoneylineOdds};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;

const ODDS_API_BASE_URL: &str = "https://api.the-odds-api.com/v4";
const SPORT_KEY: &str = "americanfootball_ncaaf"; // College football

/// Response from The Odds API for a single game
#[derive(Debug, Deserialize)]
struct OddsApiGame {
    id: String,
    sport_title: String,
    commence_time: DateTime<Utc>,
    home_team: String,
    away_team: String,
    bookmakers: Vec<OddsApiBookmaker>,
}

/// Bookmaker data from The Odds API
#[derive(Debug, Deserialize)]
struct OddsApiBookmaker {
    key: String,
    title: String,
    last_update: DateTime<Utc>,
    markets: Vec<OddsApiMarket>,
}

/// Market data (e.g., moneyline, spread) from The Odds API
#[derive(Debug, Deserialize)]
struct OddsApiMarket {
    key: String,
    outcomes: Vec<OddsApiOutcome>,
}

/// Outcome data for a specific team
#[derive(Debug, Deserialize)]
struct OddsApiOutcome {
    name: String,
    price: f64,
}

pub struct OddsApiClient {
    api_key: String,
    client: reqwest::Client,
}

impl OddsApiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// Fetch upcoming college football games with odds
    /// Only returns games that are in the future and within the next 7 days
    pub async fn fetch_games(&self) -> Result<Vec<(Game, Vec<BettingOdds>)>> {
        let url = format!("{}/sports/{}/odds", ODDS_API_BASE_URL, SPORT_KEY);

        let response = self
            .client
            .get(&url)
            .query(&[
                ("apiKey", self.api_key.as_str()),
                ("regions", "us"),
                ("markets", "h2h"), // h2h = head-to-head (moneyline)
                ("oddsFormat", "american"),
            ])
            .send()
            .await
            .context("Failed to fetch odds from The Odds API")?;

        if !response.status().is_success() {
            anyhow::bail!("Odds API returned error: {}", response.status());
        }

        let api_games: Vec<OddsApiGame> = response
            .json()
            .await
            .context("Failed to parse Odds API response")?;

        // Filter games to only include those in the future and within the next week
        let now = Utc::now();
        let one_week_from_now = now + chrono::Duration::days(7);

        Ok(api_games
            .into_iter()
            .filter(|api_game| {
                // Only include games that start in the future and within the next 7 days
                api_game.commence_time > now && api_game.commence_time <= one_week_from_now
            })
            .map(|api_game| {
                let game = Game {
                    id: api_game.id.clone(),
                    home_team: api_game.home_team,
                    away_team: api_game.away_team,
                    commence_time: api_game.commence_time,
                    sport_title: api_game.sport_title,
                };

                let odds: Vec<BettingOdds> = api_game
                    .bookmakers
                    .into_iter()
                    .filter_map(|bookmaker| {
                        // Find the moneyline market
                        let moneyline_market = bookmaker.markets.iter().find(|m| m.key == "h2h")?;

                        let moneyline: Vec<MoneylineOdds> = moneyline_market
                            .outcomes
                            .iter()
                            .map(|outcome| MoneylineOdds {
                                team: outcome.name.clone(),
                                price: outcome.price as i32,
                            })
                            .collect();

                        Some(BettingOdds {
                            game_id: api_game.id.clone(),
                            bookmaker: bookmaker.title,
                            last_update: bookmaker.last_update,
                            moneyline,
                        })
                    })
                    .collect();

                (game, odds)
            })
            .collect())
    }

    /// Check how many API requests you have remaining
    pub async fn check_usage(&self) -> Result<()> {
        let url = format!("{}/sports", ODDS_API_BASE_URL);

        let response = self
            .client
            .get(&url)
            .query(&[("apiKey", self.api_key.as_str())])
            .send()
            .await?;

        if let Some(remaining) = response.headers().get("x-requests-remaining") {
            println!("API requests remaining: {:?}", remaining);
        }

        if let Some(used) = response.headers().get("x-requests-used") {
            println!("API requests used: {:?}", used);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_fetch_games() {
        dotenv::dotenv().ok();
        let api_key = std::env::var("ODDS_API_KEY").expect("ODDS_API_KEY not set");
        let client = OddsApiClient::new(api_key);

        let games = client.fetch_games().await.unwrap();
        assert!(!games.is_empty());
    }
}
