mod api;
mod models;
mod scrapers;
mod utils;

use anyhow::Result;
use api::odds_api::OddsApiClient;
use scrapers::prediction_tracker::PredictionTrackerScraper;
use utils::ev_analysis::find_top_ev_bets;

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

    // Find top EV bets
    match find_top_ev_bets(&odds_client, &prediction_scraper, 50).await {
        Ok(bets) => {
            if bets.is_empty() {
                println!("No positive EV bets found.");
            } else {
                println!("Top {} EV Bets:\n", bets.len());
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

    // Check API usage
    println!("\n");
    odds_client.check_usage().await?;

    Ok(())
}
