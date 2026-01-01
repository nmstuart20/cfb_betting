use anyhow::{Context, Result};
use cfb_betting_ev::arbitrage::{find_moneyline_arbitrage, find_spread_arbitrage};
use cfb_betting_ev::data::{
    load_from_cache, load_moneyline_bets_from_csv, load_spread_bets_from_csv,
    save_moneyline_arbitrage_to_csv, save_moneyline_bets_to_csv, save_spread_arbitrage_to_csv,
    save_spread_bets_to_csv, save_to_cache,
};
use cfb_betting_ev::ev_analysis::{
    compare_ev_bets_to_results, compare_spread_ev_bets_to_results, find_top_ev_bets,
    find_top_spread_ev_bets,
};
use cfb_betting_ev::{
    BettingOdds, Game, GameResultsApiClient, KalshiClient, OddsApiClient, PredictionTrackerScraper,
    Sport,
};
use chrono::{Datelike, Local};
use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(name = "cfb-betting")]
#[command(about = "College Football Betting EV Calculator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Check API usage for Odds API and/or College Football Data API
    CheckUsage {
        /// Check Odds API usage
        #[arg(long)]
        odds: bool,

        /// Check College Football Data API usage
        #[arg(long)]
        cfb_data: bool,
    },
    /// Compare saved bet recommendations with actual game results
    CompareBets {
        /// Path to moneyline bets CSV file
        #[arg(long, default_value = "cache/moneyline_bets.csv")]
        moneyline_csv: String,

        /// Path to spread bets CSV file
        #[arg(long, default_value = "cache/spread_bets.csv")]
        spread_csv: String,

        /// Year of the games (defaults to current year)
        #[arg(long)]
        year: Option<u32>,

        /// Week of the games (defaults to current week)
        #[arg(long)]
        week: Option<u8>,
    },
    /// Run the full betting analysis (default)
    Analyze,
}

/// Merge Kalshi odds into existing games
/// For each Kalshi game, find a matching game in the existing list and append Kalshi odds
/// If no match is found, add the Kalshi game as a new entry
fn merge_kalshi_odds(
    existing_games: &mut Vec<(Game, Vec<BettingOdds>)>,
    kalshi_games: Vec<(Game, Vec<BettingOdds>)>,
) {
    use cfb_betting_ev::kalshi_api::normalize_team_name;

    for (kalshi_game, kalshi_odds_list) in kalshi_games {
        // Try to find a matching game in the existing list
        let kalshi_home_normalized = normalize_team_name(&kalshi_game.home_team);
        let kalshi_away_normalized = normalize_team_name(&kalshi_game.away_team);

        let mut found_match = false;
        for (existing_game, existing_odds_list) in existing_games.iter_mut() {
            let existing_home_normalized = normalize_team_name(&existing_game.home_team);
            let existing_away_normalized = normalize_team_name(&existing_game.away_team);

            // Check if teams match (in either order)
            let teams_match = (kalshi_home_normalized == existing_home_normalized
                && kalshi_away_normalized == existing_away_normalized)
                || (kalshi_home_normalized == existing_away_normalized
                    && kalshi_away_normalized == existing_home_normalized);

            if teams_match {
                // Merge Kalshi odds into this game
                existing_odds_list.extend(kalshi_odds_list.clone());
                found_match = true;
                break;
            }
        }

        if !found_match {
            // No matching game found, add as new entry
            existing_games.push((kalshi_game, kalshi_odds_list));
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Initialize logging
    tracing_subscriber::fmt::init();

    match cli.command {
        Some(Commands::CheckUsage { odds, cfb_data }) => {
            // If no flags are provided, check both by default
            let check_odds = odds || !cfb_data;
            let check_cfb = cfb_data || !odds;

            if check_odds {
                let odds_api_key =
                    std::env::var("ODDS_API_KEY").expect("ODDS_API_KEY not set in .env file");
                let odds_client = OddsApiClient::new(odds_api_key);
                println!("Checking Odds API usage...\n");
                odds_client.check_usage().await?;
                println!();
            }

            if check_cfb {
                let cfb_api_key = std::env::var("COLLEGE_FOOTBALL_DATA_API_KEY")
                    .expect("COLLEGE_FOOTBALL_DATA_API_KEY not set in .env file");
                let cfb_client = GameResultsApiClient::new(cfb_api_key);
                println!("Checking College Football Data API usage...\n");
                cfb_client.check_usage().await?;
            }

            return Ok(());
        }
        Some(Commands::CompareBets {
            moneyline_csv,
            spread_csv,
            year,
            week,
        }) => {
            println!("Comparing bet recommendations with game results...\n");

            // Get year and week (default to current if not specified)
            let now = Local::now();
            let year = year.unwrap_or(now.year() as u32);
            let week = week.unwrap_or(now.iso_week().week() as u8);

            println!("Fetching game results for week {} of {}...\n", week, year);

            // Fetch game results
            let cfb_api_key = std::env::var("COLLEGE_FOOTBALL_DATA_API_KEY")
                .expect("COLLEGE_FOOTBALL_DATA_API_KEY not set in .env file");
            let cfb_client = GameResultsApiClient::new(cfb_api_key);
            let game_results = cfb_client
                .fetch_cfb_game_results(year, week)
                .await
                .context("Failed to fetch CFB game results")?;

            println!("Fetched {} completed games\n", game_results.len());

            // Load bets from CSV files
            let moneyline_bets = if Path::new(&moneyline_csv).exists() {
                println!("Loading moneyline bets from {}...", moneyline_csv);
                load_moneyline_bets_from_csv(&moneyline_csv)?
            } else {
                println!(
                    "Moneyline CSV file not found: {}. Skipping moneyline comparison.",
                    moneyline_csv
                );
                Vec::new()
            };

            let spread_bets = if Path::new(&spread_csv).exists() {
                println!("Loading spread bets from {}...", spread_csv);
                load_spread_bets_from_csv(&spread_csv)?
            } else {
                println!(
                    "Spread CSV file not found: {}. Skipping spread comparison.",
                    spread_csv
                );
                Vec::new()
            };

            // Compare bets with results
            if !moneyline_bets.is_empty() {
                println!("\n=== MONEYLINE BET RESULTS ===\n");
                let bet_results = compare_ev_bets_to_results(&moneyline_bets, &game_results);

                let mut total_wins = 0;
                let mut total_losses = 0;
                let mut total_payout = 0.0;
                let mut total_bet = 0.0;

                for (i, result) in bet_results.iter().enumerate() {
                    println!("{}. {}", i + 1, result.format());

                    if let (Some(won), Some(payout)) = (result.bet_won, result.actual_payout) {
                        if won {
                            total_wins += 1;
                            total_payout += payout;
                        } else {
                            total_losses += 1;
                        }
                        total_bet += 1.0;
                    }
                }

                if total_bet > 0.0 {
                    let net_profit = total_payout - total_losses as f64;
                    let roi = (net_profit / total_bet) * 100.0;
                    println!("\n--- Moneyline Summary ---");
                    println!("Total Bets Resolved: {}", total_bet as i32);
                    println!(
                        "Wins: {} ({:.1}%)",
                        total_wins,
                        (total_wins as f64 / total_bet) * 100.0
                    );
                    println!(
                        "Losses: {} ({:.1}%)",
                        total_losses,
                        (total_losses as f64 / total_bet) * 100.0
                    );
                    println!("Net Profit: ${:.2}", net_profit);
                    println!("ROI: {:.2}%", roi);
                }
            }

            if !spread_bets.is_empty() {
                println!("\n=== SPREAD BET RESULTS ===\n");
                let spread_results = compare_spread_ev_bets_to_results(&spread_bets, &game_results);

                let mut total_wins = 0;
                let mut total_losses = 0;
                let mut total_payout = 0.0;
                let mut total_bet = 0.0;

                for (i, result) in spread_results.iter().enumerate() {
                    println!("{}. {}", i + 1, result.format());

                    if let (Some(won), Some(payout)) = (result.bet_won, result.actual_payout) {
                        if won {
                            total_wins += 1;
                            total_payout += payout;
                        } else {
                            total_losses += 1;
                        }
                        total_bet += 1.0;
                    }
                }

                if total_bet > 0.0 {
                    let net_profit = total_payout - total_losses as f64;
                    let roi = (net_profit / total_bet) * 100.0;
                    println!("\n--- Spread Summary ---");
                    println!("Total Bets Resolved: {}", total_bet as i32);
                    println!(
                        "Wins: {} ({:.1}%)",
                        total_wins,
                        (total_wins as f64 / total_bet) * 100.0
                    );
                    println!(
                        "Losses: {} ({:.1}%)",
                        total_losses,
                        (total_losses as f64 / total_bet) * 100.0
                    );
                    println!("Net Profit: ${:.2}", net_profit);
                    println!("ROI: {:.2}%", roi);
                }
            }

            return Ok(());
        }
        Some(Commands::Analyze) | None => {
            // Run the full analysis (default behavior)
        }
    }

    println!("College Football Betting EV Calculator\n");
    println!("Fetching betting odds and model data...\n");

    // Get API key from environment
    let api_key = std::env::var("ODDS_API_KEY").expect("ODDS_API_KEY not set in .env file");

    // Create clients
    let odds_client = OddsApiClient::new(api_key);
    let prediction_scraper = PredictionTrackerScraper::new();

    // Optionally create Kalshi client if API key is available
    let kalshi_client = std::env::var("KALSHI_API_KEY")
        .ok()
        .map(KalshiClient::new);

    if kalshi_client.is_some() {
        println!("Kalshi integration enabled\n");
    }

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
        load_from_cache(predictions_cache_file)?
    } else {
        // Fetch predictions from The Prediction Tracker
        let predictions = prediction_scraper
            .fetch_game_predictions()
            .await
            .context("Failed to fetch predictions")?;
        save_to_cache(&predictions, predictions_cache_file)?;
        println!(
            "Saved predictions to cache file: {}\n",
            predictions_cache_file
        );
        predictions
    };
    // Fetch college football odds
    let mut cfb_games_with_odds = if use_cache && Path::new(odds_cache_file).exists() {
        println!("Loading odds from cache file: {}\n", odds_cache_file);
        load_from_cache(odds_cache_file)?
    } else {
        // Fetch odds from The Odds API
        let games_with_odds = odds_client
            .fetch_games(Sport::CollegeFootball)
            .await
            .context("Failed to fetch CFB odds")?;

        // Save to cache file
        save_to_cache(&games_with_odds, odds_cache_file)?;
        println!("Saved odds to cache file: {}\n", odds_cache_file);

        games_with_odds
    };

    // Fetch and merge Kalshi odds for CFB if available
    if let Some(ref kalshi) = kalshi_client {
        let kalshi_cfb_cache = "cache/kalshi_cfb_cache.json";
        let kalshi_cfb_games = if use_cache && Path::new(kalshi_cfb_cache).exists() {
            println!(
                "Loading Kalshi CFB odds from cache file: {}\n",
                kalshi_cfb_cache
            );
            load_from_cache(kalshi_cfb_cache)?
        } else {
            match kalshi.fetch_games(Sport::CollegeFootball).await {
                Ok(games) => {
                    save_to_cache(&games, kalshi_cfb_cache)?;
                    println!(
                        "Saved Kalshi CFB odds to cache file: {}\n",
                        kalshi_cfb_cache
                    );
                    games
                }
                Err(e) => {
                    eprintln!("Warning: Failed to fetch Kalshi CFB odds: {}\n", e);
                    Vec::new()
                }
            }
        };

        if !kalshi_cfb_games.is_empty() {
            println!(
                "Merging {} Kalshi CFB games with existing odds\n",
                kalshi_cfb_games.len()
            );
            merge_kalshi_odds(&mut cfb_games_with_odds, kalshi_cfb_games);
        }
    }

    // Fetch college basketball odds
    let cbb_cache_file = "cache/cbb_odds_cache.json";
    let mut cbb_games_with_odds = if use_cache && Path::new(cbb_cache_file).exists() {
        println!("Loading CBB odds from cache file: {}\n", cbb_cache_file);
        load_from_cache(cbb_cache_file)?
    } else {
        // Fetch odds from The Odds API
        let games_with_odds = odds_client
            .fetch_games(Sport::CollegeBasketball)
            .await
            .context("Failed to fetch CBB odds")?;

        // Save to cache file
        save_to_cache(&games_with_odds, cbb_cache_file)?;
        println!("Saved CBB odds to cache file: {}\n", cbb_cache_file);

        games_with_odds
    };

    // Fetch and merge Kalshi odds for CBB if available
    if let Some(ref kalshi) = kalshi_client {
        let kalshi_cbb_cache = "cache/kalshi_cbb_cache.json";
        let kalshi_cbb_games = if use_cache && Path::new(kalshi_cbb_cache).exists() {
            println!(
                "Loading Kalshi CBB odds from cache file: {}\n",
                kalshi_cbb_cache
            );
            load_from_cache(kalshi_cbb_cache)?
        } else {
            match kalshi.fetch_games(Sport::CollegeBasketball).await {
                Ok(games) => {
                    save_to_cache(&games, kalshi_cbb_cache)?;
                    println!(
                        "Saved Kalshi CBB odds to cache file: {}\n",
                        kalshi_cbb_cache
                    );
                    games
                }
                Err(e) => {
                    eprintln!("Warning: Failed to fetch Kalshi CBB odds: {}\n", e);
                    Vec::new()
                }
            }
        };

        if !kalshi_cbb_games.is_empty() {
            println!(
                "Merging {} Kalshi CBB games with existing odds\n",
                kalshi_cbb_games.len()
            );
            merge_kalshi_odds(&mut cbb_games_with_odds, kalshi_cbb_games);
        }
    }

    // Find top moneyline EV bets (CFB only - requires predictions)
    println!("COLLEGE FOOTBALL\n");
    println!("MONEYLINE BETS\n");
    let moneyline_bets = match find_top_ev_bets(&cfb_games_with_odds, &predictions, Some(30)).await
    {
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
    let spread_bets =
        match find_top_spread_ev_bets(&cfb_games_with_odds, &predictions, Some(30)).await {
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

    // Find arbitrage opportunities for CFB
    println!("\nCFB ARBITRAGE OPPORTUNITIES\n");

    println!("MONEYLINE ARBITRAGE\n");
    let cfb_moneyline_arbs = find_moneyline_arbitrage(&cfb_games_with_odds)?;
    if cfb_moneyline_arbs.is_empty() {
        println!("No CFB moneyline arbitrage opportunities found.");
    } else {
        println!(
            "Found {} CFB Moneyline Arbitrage Opportunities:\n",
            cfb_moneyline_arbs.len()
        );
        for (i, arb) in cfb_moneyline_arbs.iter().enumerate() {
            println!("{}. {}", i + 1, arb.format());
        }
    }

    if save_csv && !cfb_moneyline_arbs.is_empty() {
        save_moneyline_arbitrage_to_csv(&cfb_moneyline_arbs, "cache/cfb_moneyline_arbitrage.csv")?;
        println!("\nSaved CFB moneyline arbitrage to cfb_moneyline_arbitrage.csv");
    }

    println!("\nSPREAD ARBITRAGE\n");
    let cfb_spread_arbs = find_spread_arbitrage(&cfb_games_with_odds)?;
    if cfb_spread_arbs.is_empty() {
        println!("No CFB spread arbitrage opportunities found.");
    } else {
        println!(
            "Found {} CFB Spread Arbitrage Opportunities:\n",
            cfb_spread_arbs.len()
        );
        for (i, arb) in cfb_spread_arbs.iter().enumerate() {
            println!("{}. {}", i + 1, arb.format());
        }
    }

    if save_csv && !cfb_spread_arbs.is_empty() {
        save_spread_arbitrage_to_csv(&cfb_spread_arbs, "cache/cfb_spread_arbitrage.csv")?;
        println!("\nSaved CFB spread arbitrage to cfb_spread_arbitrage.csv");
    }

    // Find arbitrage opportunities for CBB
    println!("\nCOLLEGE BASKETBALL\n");
    println!("CBB ARBITRAGE OPPORTUNITIES\n");

    println!("MONEYLINE ARBITRAGE\n");
    let cbb_moneyline_arbs = find_moneyline_arbitrage(&cbb_games_with_odds)?;
    if cbb_moneyline_arbs.is_empty() {
        println!("No CBB moneyline arbitrage opportunities found.");
    } else {
        println!(
            "Found {} CBB Moneyline Arbitrage Opportunities:\n",
            cbb_moneyline_arbs.len()
        );
        for (i, arb) in cbb_moneyline_arbs.iter().enumerate() {
            println!("{}. {}", i + 1, arb.format());
        }
    }

    if save_csv && !cbb_moneyline_arbs.is_empty() {
        save_moneyline_arbitrage_to_csv(&cbb_moneyline_arbs, "cache/cbb_moneyline_arbitrage.csv")?;
        println!("\nSaved CBB moneyline arbitrage to cbb_moneyline_arbitrage.csv");
    }

    println!("\nSPREAD ARBITRAGE\n");
    let cbb_spread_arbs = find_spread_arbitrage(&cbb_games_with_odds)?;
    if cbb_spread_arbs.is_empty() {
        println!("No CBB spread arbitrage opportunities found.");
    } else {
        println!(
            "Found {} CBB Spread Arbitrage Opportunities:\n",
            cbb_spread_arbs.len()
        );
        for (i, arb) in cbb_spread_arbs.iter().enumerate() {
            println!("{}. {}", i + 1, arb.format());
        }
    }

    if save_csv && !cbb_spread_arbs.is_empty() {
        save_spread_arbitrage_to_csv(&cbb_spread_arbs, "cache/cbb_spread_arbitrage.csv")?;
        println!("\nSaved CBB spread arbitrage to cbb_spread_arbitrage.csv");
    }

    // Check API usage
    println!("\n");
    odds_client.check_usage().await?;

    Ok(())
}
