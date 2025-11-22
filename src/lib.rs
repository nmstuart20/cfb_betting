pub mod api;
pub mod models;
pub mod scrapers;
pub mod utils;

use anyhow::{Context, Result};
use api::odds_api::OddsApiClient;
use api::Sport;
use scrapers::prediction_tracker::PredictionTrackerScraper;
use serde::{Deserialize, Serialize};
use std::path::Path;
use utils::arbitrage::{
    find_moneyline_arbitrage, find_spread_arbitrage, MoneylineArbitrage, SpreadArbitrage,
};
use utils::data::{
    load_odds_from_cache, load_predictions_from_cache, save_odds_to_cache,
    save_predictions_to_cache,
};
use utils::ev_analysis::{
    find_top_ev_bets, find_top_spread_ev_bets, EvBetRecommendation, SpreadEvBetRecommendation,
};

/// All the data we want to display on the web page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BettingData {
    pub cfb_moneyline_bets: Vec<EvBetRecommendation>,
    pub cfb_spread_bets: Vec<SpreadEvBetRecommendation>,
    pub cfb_moneyline_arbs: Vec<MoneylineArbitrage>,
    pub cfb_spread_arbs: Vec<SpreadArbitrage>,
    pub cbb_moneyline_arbs: Vec<MoneylineArbitrage>,
    pub cbb_spread_arbs: Vec<SpreadArbitrage>,
}

/// Fetch all betting data from APIs or cache
pub async fn fetch_all_betting_data(use_cache: bool) -> Result<BettingData> {
    // Get API key from environment
    let api_key = std::env::var("ODDS_API_KEY").expect("ODDS_API_KEY not set in .env file");

    // Create clients
    let odds_client = OddsApiClient::new(api_key);
    let prediction_scraper = PredictionTrackerScraper::new();

    // Cache file paths
    let odds_cache_file = "cache/odds_cache.json";
    let predictions_cache_file = "cache/predictions_cache.json";
    let cbb_cache_file = "cache/cbb_odds_cache.json";

    // Fetch predictions
    let predictions = if use_cache && Path::new(predictions_cache_file).exists() {
        load_predictions_from_cache(predictions_cache_file)?
    } else {
        let predictions = prediction_scraper
            .fetch_game_predictions()
            .await
            .context("Failed to fetch predictions")?;
        save_predictions_to_cache(&predictions, predictions_cache_file)?;
        predictions
    };

    // Fetch college football odds
    let cfb_games_with_odds = if use_cache && Path::new(odds_cache_file).exists() {
        load_odds_from_cache(odds_cache_file)?
    } else {
        let games_with_odds = odds_client
            .fetch_games(Sport::CollegeFootball)
            .await
            .context("Failed to fetch CFB odds")?;
        save_odds_to_cache(&games_with_odds, odds_cache_file)?;
        games_with_odds
    };

    // Fetch college basketball odds
    let cbb_games_with_odds = if use_cache && Path::new(cbb_cache_file).exists() {
        load_odds_from_cache(cbb_cache_file)?
    } else {
        let games_with_odds = odds_client
            .fetch_games(Sport::CollegeBasketball)
            .await
            .context("Failed to fetch CBB odds")?;
        save_odds_to_cache(&games_with_odds, cbb_cache_file)?;
        games_with_odds
    };

    // Calculate EV bets and arbitrage opportunities
    let cfb_moneyline_bets = find_top_ev_bets(&cfb_games_with_odds, &predictions, 30)
        .await
        .unwrap_or_default();

    let cfb_spread_bets = find_top_spread_ev_bets(&cfb_games_with_odds, &predictions, 30)
        .await
        .unwrap_or_default();

    let cfb_moneyline_arbs = find_moneyline_arbitrage(&cfb_games_with_odds)?;
    let cfb_spread_arbs = find_spread_arbitrage(&cfb_games_with_odds)?;
    let cbb_moneyline_arbs = find_moneyline_arbitrage(&cbb_games_with_odds)?;
    let cbb_spread_arbs = find_spread_arbitrage(&cbb_games_with_odds)?;

    Ok(BettingData {
        cfb_moneyline_bets,
        cfb_spread_bets,
        cfb_moneyline_arbs,
        cfb_spread_arbs,
        cbb_moneyline_arbs,
        cbb_spread_arbs,
    })
}
