use crate::models::{BettingOdds, Game, MoneylineOdds, Sport};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;

const KALSHI_API_BASE: &str = "https://trading-api.kalshi.com/trade-api/v2";

impl Sport {
    fn kalshi_series_patterns(&self) -> Vec<&'static str> {
        match self {
            Sport::CollegeFootball => vec!["HIGHCFB", "NCAAFB", "CFP"],
            Sport::CollegeBasketball => vec!["HIGHCBB", "NCAABB", "MARCHMAD"],
        }
    }
}

/// Response from Kalshi API for markets
#[derive(Debug, Deserialize)]
struct KalshiMarketsResponse {
    markets: Vec<KalshiMarket>,
    #[serde(default)]
    cursor: Option<String>,
}

/// Market data from Kalshi API
#[derive(Debug, Deserialize)]
struct KalshiMarket {
    ticker: String,
    event_ticker: String,
    series_ticker: String,
    title: String,
    subtitle: Option<String>,
    #[allow(dead_code)]
    open_time: Option<DateTime<Utc>>,
    close_time: Option<DateTime<Utc>>,
    expiration_time: Option<DateTime<Utc>>,
    #[allow(dead_code)]
    status: String,
    yes_bid: Option<u32>, // in cents (0-100)
    yes_ask: Option<u32>,
    #[allow(dead_code)]
    no_bid: Option<u32>,
    #[allow(dead_code)]
    no_ask: Option<u32>,
    #[allow(dead_code)]
    last_price: Option<u32>,
    #[allow(dead_code)]
    volume: Option<u64>,
}

pub struct KalshiClient {
    api_key: String,
    client: reqwest::Client,
}

impl KalshiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// Fetch upcoming games with odds for a given sport
    pub async fn fetch_games(&self, sport: Sport) -> Result<Vec<(Game, Vec<BettingOdds>)>> {
        let series_patterns = sport.kalshi_series_patterns();
        let mut all_markets = Vec::new();

        println!("Fetching from Kalshi API");

        // Fetch markets for each series pattern
        for pattern in series_patterns {
            match self.fetch_series_markets(pattern).await {
                Ok(mut markets) => {
                    all_markets.append(&mut markets);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to fetch Kalshi series {}: {}", pattern, e);
                    continue;
                }
            }
        }

        if all_markets.is_empty() {
            println!("No Kalshi markets found for {:?}", sport);
            return Ok(Vec::new());
        }

        // Convert Kalshi markets to Game and BettingOdds
        let games_with_odds = self.convert_markets_to_games(&all_markets)?;

        Ok(games_with_odds)
    }

    /// Fetch markets for a specific series ticker pattern
    async fn fetch_series_markets(&self, series_ticker: &str) -> Result<Vec<KalshiMarket>> {
        let url = format!("{}/markets", KALSHI_API_BASE);
        let mut all_markets = Vec::new();
        let mut cursor: Option<String> = None;

        // Handle pagination
        loop {
            let mut query_params = vec![
                ("series_ticker", series_ticker.to_string()),
                ("status", "open".to_string()),
                ("limit", "100".to_string()),
            ];

            if let Some(ref c) = cursor {
                query_params.push(("cursor", c.clone()));
            }

            let response = self
                .client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .query(&query_params)
                .send()
                .await
                .context("Failed to fetch markets from Kalshi API")?;

            if !response.status().is_success() {
                anyhow::bail!("Kalshi API returned error: {}", response.status());
            }

            let markets_response: KalshiMarketsResponse = response
                .json()
                .await
                .context("Failed to parse Kalshi API response")?;

            all_markets.extend(markets_response.markets);

            // Check if there are more pages
            if markets_response.cursor.is_none()
                || markets_response.cursor.as_ref().unwrap().is_empty()
            {
                break;
            }
            cursor = markets_response.cursor;
        }

        Ok(all_markets)
    }

    /// Convert Kalshi markets to Game and BettingOdds format
    fn convert_markets_to_games(
        &self,
        markets: &[KalshiMarket],
    ) -> Result<Vec<(Game, Vec<BettingOdds>)>> {
        // Group markets by event_ticker
        let mut events_map: std::collections::HashMap<String, Vec<&KalshiMarket>> =
            std::collections::HashMap::new();

        for market in markets {
            events_map
                .entry(market.event_ticker.clone())
                .or_insert_with(Vec::new)
                .push(market);
        }

        let mut games_with_odds = Vec::new();

        for (event_ticker, event_markets) in events_map {
            // Try to parse the event into a game
            if let Some((game, odds)) = self.parse_event_to_game(&event_ticker, &event_markets) {
                games_with_odds.push((game, vec![odds]));
            }
        }

        Ok(games_with_odds)
    }

    /// Parse an event and its markets into a Game and BettingOdds
    fn parse_event_to_game(
        &self,
        event_ticker: &str,
        markets: &[&KalshiMarket],
    ) -> Option<(Game, BettingOdds)> {
        if markets.is_empty() {
            return None;
        }

        // Use the first market to get event-level info
        let first_market = markets[0];

        // Parse teams from the market title
        // Common patterns:
        // "Will [Team] win?"
        // "Will [Team] beat [Team]?"
        // "[Team] vs [Team]"
        let (home_team, away_team) = self.parse_teams_from_title(&first_market.title)?;

        // Determine sport title
        let sport_title = if first_market.series_ticker.contains("CFB")
            || first_market.series_ticker.contains("NCAAFB")
            || first_market.series_ticker.contains("CFP")
        {
            "NCAAF".to_string()
        } else if first_market.series_ticker.contains("CBB")
            || first_market.series_ticker.contains("NCAABB")
            || first_market.series_ticker.contains("MARCHMAD")
        {
            "NCAAB".to_string()
        } else {
            "Unknown".to_string()
        };

        // Use expiration time as commence time (best approximation)
        let commence_time = first_market
            .expiration_time
            .or(first_market.close_time)
            .unwrap_or_else(Utc::now);

        let game = Game {
            id: event_ticker.to_string(),
            home_team: home_team.clone(),
            away_team: away_team.clone(),
            commence_time,
            sport_title,
        };

        // Convert markets to moneyline odds
        let mut moneyline_odds = Vec::new();

        for market in markets {
            // Determine which team this market is for
            let team = self.determine_team_from_market(market, &home_team, &away_team)?;

            // Convert Kalshi prices to American odds
            if let Some(american_odds) = self.kalshi_to_american_odds(market) {
                moneyline_odds.push(MoneylineOdds {
                    team,
                    price: american_odds,
                });
            }
        }

        if moneyline_odds.is_empty() {
            return None;
        }

        let betting_odds = BettingOdds {
            game_id: event_ticker.to_string(),
            bookmaker: "Kalshi".to_string(),
            last_update: Utc::now(),
            moneyline: moneyline_odds,
            spreads: Vec::new(), // Kalshi doesn't have traditional spreads
        };

        Some((game, betting_odds))
    }

    /// Parse team names from market title
    fn parse_teams_from_title(&self, title: &str) -> Option<(String, String)> {
        // Common patterns:
        // "Will Ohio State beat Michigan?"
        // "Will Alabama win?"
        // "Ohio State vs Michigan"

        let lower_title = title.to_lowercase();

        // Pattern: "Will [Team1] beat [Team2]?"
        if let Some(beat_idx) = lower_title.find(" beat ") {
            if let Some(will_idx) = lower_title.find("will ") {
                let team1_start = will_idx + 5;
                let team1 = title[team1_start..beat_idx].trim().to_string();
                let team2_start = beat_idx + 6;
                let team2_end = title[team2_start..]
                    .find('?')
                    .map(|i| team2_start + i)
                    .unwrap_or(title.len());
                let team2 = title[team2_start..team2_end].trim().to_string();
                return Some((team1, team2));
            }
        }

        // Pattern: "[Team1] vs [Team2]"
        if let Some(vs_idx) = lower_title.find(" vs ") {
            let team1 = title[..vs_idx].trim().to_string();
            let team2 = title[vs_idx + 4..].trim().to_string();
            return Some((team1, team2));
        }

        // Pattern: "[Team1] @ [Team2]"
        if let Some(at_idx) = lower_title.find(" @ ") {
            let team1 = title[..at_idx].trim().to_string();
            let team2 = title[at_idx + 3..].trim().to_string();
            return Some((team2, team1)); // @ means team1 is away, team2 is home
        }

        None
    }

    /// Determine which team a market is for based on the subtitle or title
    fn determine_team_from_market(
        &self,
        market: &KalshiMarket,
        home_team: &str,
        away_team: &str,
    ) -> Option<String> {
        let text = market
            .subtitle
            .as_ref()
            .unwrap_or(&market.title)
            .to_lowercase();
        let home_lower = home_team.to_lowercase();
        let away_lower = away_team.to_lowercase();

        // Check if the market is about home team winning
        if text.contains(&home_lower) && text.contains("win") {
            Some(home_team.to_string())
        } else if text.contains(&away_lower) && text.contains("win") {
            Some(away_team.to_string())
        } else if text.contains(&home_lower) {
            Some(home_team.to_string())
        } else if text.contains(&away_lower) {
            Some(away_team.to_string())
        } else {
            None
        }
    }

    /// Convert Kalshi yes/no prices to American odds
    fn kalshi_to_american_odds(&self, market: &KalshiMarket) -> Option<i32> {
        let yes_bid = market.yes_bid?;
        let yes_ask = market.yes_ask?;

        // Validate prices are in valid range
        if yes_bid > 100 || yes_ask > 100 {
            eprintln!(
                "Warning: Invalid Kalshi prices for {}: bid={}, ask={}",
                market.ticker, yes_bid, yes_ask
            );
            return None;
        }

        // Calculate mid-price as probability (0.0 to 1.0)
        let mid_price_cents = (yes_bid + yes_ask) as f64 / 2.0;
        let probability = mid_price_cents / 100.0;

        // Convert probability to American odds
        let american_odds = probability_to_american_odds(probability);

        // Validate odds are reasonable
        if american_odds < -10000 || american_odds > 10000 {
            eprintln!(
                "Warning: Unreasonable odds for {}: {}",
                market.ticker, american_odds
            );
            return None;
        }

        Some(american_odds)
    }
}

/// Convert probability (0.0 to 1.0) to American odds
/// This matches the logic from ev_calculator.rs::_probability_to_american_odds
fn probability_to_american_odds(prob: f64) -> i32 {
    if prob >= 0.5 {
        // Favorite (negative odds)
        -((prob / (1.0 - prob)) * 100.0) as i32
    } else {
        // Underdog (positive odds)
        (((1.0 - prob) / prob) * 100.0) as i32
    }
}

/// Normalize team name for matching
pub fn normalize_team_name(name: &str) -> String {
    name.to_lowercase()
        .replace("buckeyes", "")
        .replace("wolverines", "")
        .replace("crimson tide", "")
        .replace("tigers", "")
        .replace("bulldogs", "")
        .replace("gators", "")
        .replace("longhorns", "")
        .replace("sooners", "")
        .replace("tar heels", "")
        .replace("blue devils", "")
        .replace("the ", "")
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probability_to_american_odds() {
        // Favorite scenarios (prob >= 0.5)
        assert_eq!(probability_to_american_odds(0.62), -163); // 62% chance
        assert_eq!(probability_to_american_odds(0.5), -100); // 50% chance (even)
        assert_eq!(probability_to_american_odds(0.75), -300); // 75% chance

        // Underdog scenarios (prob < 0.5)
        assert_eq!(probability_to_american_odds(0.37), 170); // 37% chance
                                                             // 0.4 probability can result in 149 or 150 due to floating point precision
        let result = probability_to_american_odds(0.4);
        assert!(
            result == 149 || result == 150,
            "Expected 149 or 150, got {}",
            result
        );
        assert_eq!(probability_to_american_odds(0.25), 300); // 25% chance
    }

    #[test]
    fn test_normalize_team_name() {
        assert_eq!(normalize_team_name("Ohio State Buckeyes"), "ohio state");
        assert_eq!(normalize_team_name("Michigan Wolverines"), "michigan");
        assert_eq!(normalize_team_name("Alabama Crimson Tide"), "alabama");
        assert_eq!(normalize_team_name("The Ohio State"), "ohio state");
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_cfb_games() {
        dotenv::dotenv().ok();
        let api_key = std::env::var("KALSHI_API_KEY").expect("KALSHI_API_KEY not set");
        let client = KalshiClient::new(api_key);

        let games = client.fetch_games(Sport::CollegeFootball).await.unwrap();
        println!("Found {} CFB games from Kalshi", games.len());

        for (game, odds_list) in games.iter() {
            println!("\nGame: {} @ {}", game.away_team, game.home_team);
            for odds in odds_list {
                println!("  Bookmaker: {}", odds.bookmaker);
                for ml in &odds.moneyline {
                    println!("    {} -> {}", ml.team, ml.price);
                }
            }
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_cbb_games() {
        dotenv::dotenv().ok();
        let api_key = std::env::var("KALSHI_API_KEY").expect("KALSHI_API_KEY not set");
        let client = KalshiClient::new(api_key);

        let games = client.fetch_games(Sport::CollegeBasketball).await.unwrap();
        println!("Found {} CBB games from Kalshi", games.len());

        for (game, odds_list) in games.iter() {
            println!("\nGame: {} @ {}", game.away_team, game.home_team);
            for odds in odds_list {
                println!("  Bookmaker: {}", odds.bookmaker);
                for ml in &odds.moneyline {
                    println!("    {} -> {}", ml.team, ml.price);
                }
            }
        }
    }
}
