pub mod api;
pub mod models;
pub mod scrapers;
pub mod utils;

pub use api::*;
pub use models::*;
pub use scrapers::*;
pub use utils::*;

use anyhow::{Context, Result};
use api::game_results_api::{CbbGameResult, GameResult, GameResultsApiClient};
use api::odds_api::OddsApiClient;
use chrono::prelude::*;
use scrapers::prediction_tracker::PredictionTrackerScraper;
use serde::{Deserialize, Serialize};
use std::path::Path;
use utils::arbitrage::{
    find_moneyline_arbitrage, find_spread_arbitrage, MoneylineArbitrage, SpreadArbitrage,
};
use utils::data::{load_from_cache, save_to_cache};
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
    pub cfb_game_results: Vec<GameResult>,
    pub cbb_game_results: Vec<CbbGameResult>,
}

/// Fetch all betting data from APIs or cache
pub async fn fetch_all_betting_data(use_cache: bool) -> Result<BettingData> {
    // Load .env file
    dotenv::dotenv().ok();

    // Get API key from environment
    let odds_api_key = std::env::var("ODDS_API_KEY").expect("ODDS_API_KEY not set in .env file");
    let cfb_api_key = std::env::var("COLLEGE_FOOTBALL_DATA_API_KEY")
        .expect("COLLEGE_FOOTBALL_DATA_API_KEY not set in .env file");

    // Create clients
    let odds_client = OddsApiClient::new(odds_api_key);
    let prediction_scraper = PredictionTrackerScraper::new();
    let game_results_client = GameResultsApiClient::new(cfb_api_key);

    // Cache file paths
    let odds_cache_file = "cache/odds_cache.json";
    let predictions_cache_file = "cache/predictions_cache.json";
    let cbb_cache_file = "cache/cbb_odds_cache.json";
    let cfb_results_cache_file = "cache/cfb_results_cache.json";
    let cbb_results_cache_file = "cache/cbb_results_cache.json";

    // Fetch predictions
    let predictions = if use_cache && Path::new(predictions_cache_file).exists() {
        load_from_cache(predictions_cache_file)?
    } else {
        let predictions = prediction_scraper
            .fetch_game_predictions()
            .await
            .context("Failed to fetch predictions")?;
        save_to_cache(&predictions, predictions_cache_file)?;
        predictions
    };

    // Fetch college football odds
    let cfb_games_with_odds = if use_cache && Path::new(odds_cache_file).exists() {
        load_from_cache(odds_cache_file)?
    } else {
        let games_with_odds = odds_client
            .fetch_games(Sport::CollegeFootball)
            .await
            .context("Failed to fetch CFB odds")?;
        save_to_cache(&games_with_odds, odds_cache_file)?;
        games_with_odds
    };

    // Fetch college basketball odds
    let cbb_games_with_odds = if use_cache && Path::new(cbb_cache_file).exists() {
        load_from_cache(cbb_cache_file)?
    } else {
        let games_with_odds = odds_client
            .fetch_games(Sport::CollegeBasketball)
            .await
            .context("Failed to fetch CBB odds")?;
        save_to_cache(&games_with_odds, cbb_cache_file)?;
        games_with_odds
    };

    // Fetch college football game results
    let cfb_game_results = if use_cache && Path::new(cfb_results_cache_file).exists() {
        load_from_cache(cfb_results_cache_file)?
    } else {
        let now = Local::now();
        let year = now.year() as u32;
        let week = now.iso_week().week() as u8;
        let game_results = game_results_client
            .fetch_cfb_game_results(year, week)
            .await
            .context("Failed to fetch CFB game results")?;
        save_to_cache(&game_results, cfb_results_cache_file)?;
        game_results
    };

    // Fetch college basketball game results
    // let cbb_game_results = if use_cache && Path::new(cbb_results_cache_file).exists() {
    //     load_from_cache(cbb_results_cache_file)?
    // } else {
    //     let now = Local::now();
    //     let day = now.format("%Y-%m-%d").to_string();
    //     let game_results = game_results_client
    //         .fetch_cbb_game_results(&day)
    //         .await
    //         .context("Failed to fetch CBB game results")?;
    //     save_to_cache(&game_results, cbb_results_cache_file)?;
    //     game_results
    // };
    let cbb_game_results = vec![];

    // Calculate EV bets and arbitrage opportunities (None = all positive EV bets)
    let cfb_moneyline_bets = find_top_ev_bets(&cfb_games_with_odds, &predictions, None)
        .await
        .unwrap_or_default();

    let cfb_spread_bets = find_top_spread_ev_bets(&cfb_games_with_odds, &predictions, None)
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
        cfb_game_results,
        cbb_game_results,
    })
}
