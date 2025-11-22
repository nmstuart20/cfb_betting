use crate::models::{BettingOdds, Game};
use crate::scrapers::prediction_tracker::GamePrediction;
use crate::utils::arbitrage::{MoneylineArbitrage, SpreadArbitrage};
use crate::{EvBetRecommendation, SpreadEvBetRecommendation};
use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;

/// Save odds data to a JSON cache file
pub fn save_odds_to_cache(
    games_with_odds: &[(Game, Vec<BettingOdds>)],
    odds_cache_file: &str,
) -> Result<()> {
    let json =
        serde_json::to_string_pretty(games_with_odds).context("Failed to serialize odds data")?;
    std::fs::write(odds_cache_file, json).context("Failed to write cache file")?;
    Ok(())
}

/// Save moneyline arbitrage opportunities to CSV
pub fn save_moneyline_arbitrage_to_csv(arbs: &[MoneylineArbitrage], filename: &str) -> Result<()> {
    let mut file = File::create(filename).context("Failed to create CSV file")?;

    // Write CSV header
    writeln!(
        file,
        "Home Team,Away Team,Home Bookmaker,Home Odds,Home Stake %,Away Bookmaker,Away Odds,Away Stake %,Profit %"
    )?;

    // Write each arbitrage opportunity
    for arb in arbs {
        writeln!(
            file,
            "{},{},{},{},{:.2},{},{},{:.2},{:.2}",
            arb.home_team,
            arb.away_team,
            arb.home_bookmaker,
            arb.home_odds,
            arb.home_stake_percentage,
            arb.away_bookmaker,
            arb.away_odds,
            arb.away_stake_percentage,
            arb.profit_percentage
        )?;
    }

    Ok(())
}

/// Save spread arbitrage opportunities to CSV
pub fn save_spread_arbitrage_to_csv(arbs: &[SpreadArbitrage], filename: &str) -> Result<()> {
    let mut file = File::create(filename).context("Failed to create CSV file")?;

    // Write CSV header
    writeln!(
        file,
        "Home Team,Away Team,Side 1 Team,Side 1 Spread,Side 1 Odds,Side 1 Bookmaker,Side 1 Stake %,Side 2 Team,Side 2 Spread,Side 2 Odds,Side 2 Bookmaker,Side 2 Stake %,Profit %"
    )?;

    // Write each arbitrage opportunity
    for arb in arbs {
        writeln!(
            file,
            "{},{},{},{:.1},{},{},{:.2},{},{:.1},{},{},{:.2},{:.2}",
            arb.home_team,
            arb.away_team,
            arb.side1_team,
            arb.side1_spread,
            arb.side1_odds,
            arb.side1_bookmaker,
            arb.side1_stake_percentage,
            arb.side2_team,
            arb.side2_spread,
            arb.side2_odds,
            arb.side2_bookmaker,
            arb.side2_stake_percentage,
            arb.profit_percentage
        )?;
    }

    Ok(())
}

pub fn save_predictions_to_cache(
    predictions: &[GamePrediction],
    predictions_cache: &str,
) -> Result<()> {
    let json =
        serde_json::to_string_pretty(predictions).context("Failed to serialize prediction data")?;
    std::fs::write(predictions_cache, json)?;
    Ok(())
}

/// Load odds data from a JSON cache file
pub fn load_odds_from_cache(odds_cache_file: &str) -> Result<Vec<(Game, Vec<BettingOdds>)>> {
    let json = std::fs::read_to_string(odds_cache_file).context("Failed to read cache file")?;
    let games_with_odds: Vec<(Game, Vec<BettingOdds>)> =
        serde_json::from_str(&json).context("Failed to deserialize odds data")?;
    Ok(games_with_odds)
}

/// Load prediction data from JSON
pub fn load_predictions_from_cache(predictions_cache_file: &str) -> Result<Vec<GamePrediction>> {
    let json =
        std::fs::read_to_string(predictions_cache_file).context("Failed to read cache file")?;
    let predictions: Vec<GamePrediction> =
        serde_json::from_str(&json).context("Failed to deserialize prediction data")?;
    Ok(predictions)
}

/// Save moneyline bets to CSV
pub fn save_moneyline_bets_to_csv(bets: &[EvBetRecommendation], filename: &str) -> Result<()> {
    let mut file = File::create(filename).context("Failed to create CSV file")?;

    // Write CSV header
    writeln!(
        file,
        "Home Team,Away Team,Bet Team,Odds,Bookmaker,Expected Value (%),Edge (%),Model Probability (%),Implied Probability (%)"
    )?;

    // Write each bet
    for bet in bets {
        writeln!(
            file,
            "{},{},{},{},{},{:.2},{:.2},{:.1},{:.1}",
            bet.home_team,
            bet.away_team,
            bet.team,
            bet.odds,
            bet.bookmaker,
            bet.expected_value * 100.0,
            bet.edge * 100.0,
            bet.model_prob * 100.0,
            bet.implied_prob * 100.0
        )?;
    }

    Ok(())
}

/// Save spread bets to CSV
pub fn save_spread_bets_to_csv(bets: &[SpreadEvBetRecommendation], filename: &str) -> Result<()> {
    let mut file = File::create(filename).context("Failed to create CSV file")?;

    // Write CSV header
    writeln!(
        file,
        "Home Team,Away Team,Bet Team,Spread,Odds,Bookmaker,Expected Value (%),Edge (%),Model Spread,Model Probability (%),Implied Probability (%)"
    )?;

    // Write each bet
    for bet in bets {
        writeln!(
            file,
            "{},{},{},{:.1},{},{},{:.2},{:.2},{:.1},{:.1},{:.1}",
            bet.home_team,
            bet.away_team,
            bet.team,
            bet.spread_line,
            bet.odds,
            bet.bookmaker,
            bet.expected_value * 100.0,
            bet.edge * 100.0,
            bet.model_spread,
            bet.model_prob * 100.0,
            bet.implied_prob * 100.0
        )?;
    }

    Ok(())
}
