use crate::api::odds_api::OddsApiClient;
use crate::models::{BettingOdds, ExpectedValue, Game};
use crate::scrapers::prediction_tracker::{normalize_team_name, PredictionTrackerScraper};
use crate::utils::ev_calculator::{american_odds_to_probability, calculate_expected_value};
use anyhow::{Context, Result};
use std::collections::HashMap;

/// Extract the primary school name from a full team name
/// "Iowa Hawkeyes" -> "iowa"
/// "Ohio State Buckeyes" -> "ohio_st" (keeping common abbreviations)
fn extract_school_name(team_name: &str) -> String {
    let normalized = normalize_team_name(team_name);

    // Split by underscore and take the first part (or first two if it's a state school)
    let parts: Vec<&str> = normalized.split('_').collect();

    if parts.len() >= 2 {
        // Check for common patterns like "ohio_st", "michigan_st", etc.
        if parts.len() >= 2 && (parts[1] == "st" || parts[1] == "state" || parts[1] == "dame") {
            format!("{}_{}", parts[0], parts[1])
        } else if parts.len() >= 3 && parts[1] == "aandm" {
            // Handle "Texas A&M" -> "texas_aandm"
            format!("{}_{}", parts[0], parts[1])
        } else {
            // Just use the first word (e.g., "iowa" from "iowa_hawkeyes")
            parts[0].to_string()
        }
    } else {
        normalized
    }
}

/// Analyze all available games and return the top N EV bets
pub async fn find_top_ev_bets(
    odds_client: &OddsApiClient,
    prediction_scraper: &PredictionTrackerScraper,
    top_n: usize,
) -> Result<Vec<EvBetRecommendation>> {
    // Fetch odds from The Odds API
    let games_with_odds = odds_client
        .fetch_games()
        .await
        .context("Failed to fetch odds")?;

    // Fetch predictions from Prediction Tracker
    let predictions = prediction_scraper
        .fetch_predictions()
        .await
        .context("Failed to fetch predictions")?;

    // Create a lookup map for predictions by team names
    // Use extract_school_name to match with Odds API which has full names
    let mut prediction_map: HashMap<String, HashMap<String, f64>> = HashMap::new();
    for pred in predictions {
        let home_key = extract_school_name(&pred.home_team);
        let away_key = extract_school_name(&pred.away_team);

        let mut game_map = HashMap::new();
        game_map.insert(home_key.clone(), pred.home_win_prob);
        game_map.insert(away_key.clone(), pred.away_win_prob);

        // Store by both team combinations
        prediction_map.insert(format!("{}_{}", home_key, away_key), game_map.clone());
        prediction_map.insert(format!("{}_{}", away_key, home_key), game_map);
    }

    // Calculate EV for all bets
    let mut all_bets = Vec::new();

    for (game, odds_list) in games_with_odds {
        // Extract school names from full team names (e.g., "Iowa Hawkeyes" -> "iowa")
        let home_key = extract_school_name(&game.home_team);
        let away_key = extract_school_name(&game.away_team);

        // Try to find matching prediction
        let game_key = format!("{}_{}", home_key, away_key);
        let game_predictions = match prediction_map.get(&game_key) {
            Some(preds) => preds,
            None => {
                println!(
                    "⚠️  No prediction found for: {} vs {} (odds api key: {})",
                    game.home_team, game.away_team, game_key
                );
                continue; // Skip games without predictions
            }
        };

        // Analyze each bookmaker's odds
        for bookmaker_odds in odds_list {
            for moneyline in &bookmaker_odds.moneyline {
                let team_key = extract_school_name(&moneyline.team);

                if let Some(&model_prob) = game_predictions.get(&team_key) {
                    let implied_prob = american_odds_to_probability(moneyline.price);
                    let ev = calculate_expected_value(model_prob, moneyline.price);
                    let edge = model_prob - implied_prob;

                    all_bets.push(EvBetRecommendation {
                        game_id: game.id.clone(),
                        home_team: game.home_team.clone(),
                        away_team: game.away_team.clone(),
                        commence_time: game.commence_time,
                        team: moneyline.team.clone(),
                        bookmaker: bookmaker_odds.bookmaker.clone(),
                        odds: moneyline.price,
                        model_prob,
                        implied_prob,
                        expected_value: ev,
                        edge,
                    });
                }
            }
        }
    }

    // Sort by EV (descending) and take top N
    all_bets.sort_by(|a, b| {
        b.expected_value
            .partial_cmp(&a.expected_value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(all_bets.into_iter().take(top_n).collect())
}

/// A bet recommendation with EV analysis
#[derive(Debug, Clone)]
pub struct EvBetRecommendation {
    pub game_id: String,
    pub home_team: String,
    pub away_team: String,
    pub commence_time: chrono::DateTime<chrono::Utc>,
    pub team: String,
    pub bookmaker: String,
    pub odds: i32,
    pub model_prob: f64,
    pub implied_prob: f64,
    pub expected_value: f64,
    pub edge: f64,
}

impl EvBetRecommendation {
    /// Format the bet recommendation as a readable string
    pub fn format(&self) -> String {
        format!(
            "{} @ {} | Bet: {} ({:+}) on {} | EV: {:+.2}% | Edge: {:+.2}% | Model: {:.1}% | Implied: {:.1}%",
            self.away_team,
            self.home_team,
            self.team,
            self.odds,
            self.bookmaker,
            self.expected_value * 100.0,
            self.edge * 100.0,
            self.model_prob * 100.0,
            self.implied_prob * 100.0
        )
    }
}
