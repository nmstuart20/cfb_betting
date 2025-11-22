mod api;
mod models;
mod scrapers;
mod utils;

use anyhow::{Context, Result};
use api::odds_api::OddsApiClient;
use scrapers::prediction_tracker::PredictionTrackerScraper;
use std::path::Path;
use utils::arbitrage::{find_moneyline_arbitrage, find_spread_arbitrage};
use utils::data::{
    load_odds_from_cache, load_predictions_from_cache, save_moneyline_arbitrage_to_csv,
    save_moneyline_bets_to_csv, save_odds_to_cache, save_predictions_to_cache,
    save_spread_arbitrage_to_csv, save_spread_bets_to_csv,
};
use utils::ev_analysis::{
    find_top_ev_bets, find_top_spread_ev_bets, EvBetRecommendation, SpreadEvBetRecommendation,
};

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
    let save_csv = std::env::var("SAVE_CSV").unwrap_or_default() == "1";

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
    let moneyline_bets = match find_top_ev_bets(&games_with_odds, &predictions, 30).await {
        Ok(bets) => {
            if bets.is_empty() {
                println!("No positive EV moneyline bets found.");
            } else {
                println!("Top {} Moneyline EV Bets:\n", bets.len());
                for (i, bet) in bets.iter().enumerate() {
                    println!("{}. {}", i + 1, bet.format());
                }
            }
            bets
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            return Err(e);
        }
    };

    if save_csv && !moneyline_bets.is_empty() {
        save_moneyline_bets_to_csv(&moneyline_bets, "cache/moneyline_bets.csv")?;
        println!("\nSaved moneyline bets to moneyline_bets.csv");
    }

    // Find top spread EV bets
    println!("\nSPREAD BETS\n");
    let spread_bets = match find_top_spread_ev_bets(&games_with_odds, &predictions, 30).await {
        Ok(bets) => {
            if bets.is_empty() {
                println!("No positive EV spread bets found.");
            } else {
                println!("Top {} Spread EV Bets:\n", bets.len());
                for (i, bet) in bets.iter().enumerate() {
                    println!("{}. {}", i + 1, bet.format());
                }
            }
            bets
        }
        Err(e) => {
            eprintln!("Error fetching spread bets: {}", e);
            // Don't return error - still show API usage
            Vec::new()
        }
    };

    if save_csv && !spread_bets.is_empty() {
        save_spread_bets_to_csv(&spread_bets, "cache/spread_bets.csv")?;
        println!("\nSaved spread bets to spread_bets.csv");
    }

    // Find arbitrage opportunities
    println!("\nARBITRAGE OPPORTUNITIES\n");

    println!("MONEYLINE ARBITRAGE\n");
    let moneyline_arbs = find_moneyline_arbitrage(&games_with_odds)?;
    if moneyline_arbs.is_empty() {
        println!("No moneyline arbitrage opportunities found.");
    } else {
        println!(
            "Found {} Moneyline Arbitrage Opportunities:\n",
            moneyline_arbs.len()
        );
        for (i, arb) in moneyline_arbs.iter().enumerate() {
            println!("{}. {}", i + 1, arb.format());
        }
    }

    if save_csv && !moneyline_arbs.is_empty() {
        save_moneyline_arbitrage_to_csv(&moneyline_arbs, "cache/moneyline_arbitrage.csv")?;
        println!("\nSaved moneyline arbitrage to moneyline_arbitrage.csv");
    }

    println!("\nSPREAD ARBITRAGE\n");
    let spread_arbs = find_spread_arbitrage(&games_with_odds)?;
    if spread_arbs.is_empty() {
        println!("No spread arbitrage opportunities found.");
    } else {
        println!(
            "Found {} Spread Arbitrage Opportunities:\n",
            spread_arbs.len()
        );
        for (i, arb) in spread_arbs.iter().enumerate() {
            println!("{}. {}", i + 1, arb.format());
        }
    }

    if save_csv && !spread_arbs.is_empty() {
        save_spread_arbitrage_to_csv(&spread_arbs, "cache/spread_arbitrage.csv")?;
        println!("\nSaved spread arbitrage to spread_arbitrage.csv");
    }

    // Check API usage
    println!("\n");
    odds_client.check_usage().await?;

    Ok(())
}
