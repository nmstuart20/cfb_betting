use crate::utils::arbitrage::{MoneylineArbitrage, SpreadArbitrage};
use crate::{EvBetRecommendation, SpreadEvBetRecommendation};
use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Save any serializable data to a JSON cache file.
pub fn save_to_cache<T: Serialize>(data: &T, cache_file: &str) -> Result<()> {
    let json = serde_json::to_string_pretty(data).context("Failed to serialize data")?;
    std::fs::create_dir_all(Path::new(cache_file).parent().unwrap())?;
    std::fs::write(cache_file, json).context("Failed to write cache file")?;
    Ok(())
}

/// Load any deserializable data from a JSON cache file.
pub fn load_from_cache<T: DeserializeOwned>(cache_file: &str) -> Result<T> {
    let json = std::fs::read_to_string(cache_file).context("Failed to read cache file")?;
    let data: T = serde_json::from_str(&json).context("Failed to deserialize data")?;
    Ok(data)
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

