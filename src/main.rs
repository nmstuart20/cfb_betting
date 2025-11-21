mod api;
mod models;
mod scrapers;
mod utils;

use anyhow::{Context, Result};
use api::odds_api::OddsApiClient;
use models::{BettingOdds, Game};
use scrapers::prediction_tracker::PredictionTrackerScraper;
use std::path::Path;
use utils::ev_analysis::{find_top_ev_bets, find_top_spread_ev_bets};

use crate::scrapers::prediction_tracker::GamePrediction;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("College Football Betting EV Calculator\n");
    println!("Fetching betting odds and model data...\n");

    // Get API key from environment
    let api_key = std::env::var("ODDS_API_KEY").expect("ODDS_API_KEY not set in .env file");

    // Create clients
    let odds_client = OddsApiClient::new(api_key);
    let prediction_scraper = PredictionTrackerScraper::new();

    // Check if we should use cached data
    let odds_cache_file = "cache/odds_cache.json";
    let predictions_cache_file = "cache/predictions_cache.json";
    let use_cache = std::env::var("USE_CACHE").unwrap_or_default() == "1";

    let predictions = if use_cache && Path::new(predictions_cache_file).exists() {
        println!(
            "Loading predictions from cache file: {}\n",
            predictions_cache_file
        );
        load_predictions_from_cache(predictions_cache_file)?
    } else {
        // Fetch predictions from The Prediction Tracker
        let predictions = prediction_scraper
            .fetch_game_predictions()
            .await
            .context("Failed to fetch predictions")?;
        save_predictions_to_cache(&predictions, predictions_cache_file)?;
        println!(
            "Saved predictions to cache file: {}\n",
            predictions_cache_file
        );
        predictions
    };
    let games_with_odds = if use_cache && Path::new(odds_cache_file).exists() {
        println!("Loading odds from cache file: {}\n", odds_cache_file);
        load_odds_from_cache(odds_cache_file)?
    } else {
        // Fetch odds from The Odds API
        let games_with_odds = odds_client
            .fetch_games()
            .await
            .context("Failed to fetch odds")?;

        // Save to cache file
        save_odds_to_cache(&games_with_odds, odds_cache_file)?;
        println!("Saved odds to cache file: {}\n", odds_cache_file);

        games_with_odds
    };

    // Find top moneyline EV bets
    println!("MONEYLINE BETS\n");
    match find_top_ev_bets(&games_with_odds, &predictions, 30).await {
        Ok(bets) => {
            if bets.is_empty() {
                println!("No positive EV moneyline bets found.");
            } else {
                println!("Top {} Moneyline EV Bets:\n", bets.len());
                for (i, bet) in bets.iter().enumerate() {
                    println!("{}. {}", i + 1, bet.format());
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            return Err(e);
        }
    }

    // Find top spread EV bets
    println!("\nSPREAD BETS\n");
    match find_top_spread_ev_bets(&games_with_odds, &predictions, 30).await {
        Ok(bets) => {
            if bets.is_empty() {
                println!("No positive EV spread bets found.");
            } else {
                println!("Top {} Spread EV Bets:\n", bets.len());
                for (i, bet) in bets.iter().enumerate() {
                    println!("{}. {}", i + 1, bet.format());
                }
            }
        }
        Err(e) => {
            eprintln!("Error fetching spread bets: {}", e);
            // Don't return error - still show API usage
        }
    }

    // Check API usage
    println!("\n");
    odds_client.check_usage().await?;

    Ok(())
}

/// Save odds data to a JSON cache file
fn save_odds_to_cache(
    games_with_odds: &[(Game, Vec<BettingOdds>)],
    odds_cache_file: &str,
) -> Result<()> {
    let json =
        serde_json::to_string_pretty(games_with_odds).context("Failed to serialize odds data")?;
    std::fs::write(odds_cache_file, json).context("Failed to write cache file")?;
    Ok(())
}

fn save_predictions_to_cache(
    predictions: &[GamePrediction],
    predictions_cache: &str,
) -> Result<()> {
    let json =
        serde_json::to_string_pretty(predictions).context("Failed to serialize prediction data")?;
    std::fs::write(predictions_cache, json)?;
    Ok(())
}

/// Load odds data from a JSON cache file
fn load_odds_from_cache(odds_cache_file: &str) -> Result<Vec<(Game, Vec<BettingOdds>)>> {
    let json = std::fs::read_to_string(odds_cache_file).context("Failed to read cache file")?;
    let games_with_odds: Vec<(Game, Vec<BettingOdds>)> =
        serde_json::from_str(&json).context("Failed to deserialize odds data")?;
    Ok(games_with_odds)
}

/// Load prediction data from JSON
fn load_predictions_from_cache(predictions_cache_file: &str) -> Result<Vec<GamePrediction>> {
    let json =
        std::fs::read_to_string(predictions_cache_file).context("Failed to read cache file")?;
    let predictions: Vec<GamePrediction> =
        serde_json::from_str(&json).context("Failed to deserialize prediction data")?;
    Ok(predictions)
}
