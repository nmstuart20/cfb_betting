use crate::models::{BettingOdds, Game};
use crate::scrapers::prediction_tracker::{normalize_team_name, GamePrediction};
use crate::utils::ev_calculator::{
    american_odds_to_probability, calculate_expected_value, calculate_spread_cover_probability,
};
use anyhow::Result;
use std::collections::HashMap;

/// Extract the primary school name from a full team name
/// "Iowa Hawkeyes" -> "iowa"
/// "Ohio State Buckeyes" -> "ohio_st"
/// "San Diego State Aztecs" -> "san_diego_st"
fn extract_school_name(team_name: &str) -> String {
    // Apply special mappings first (matching what the scraper does)
    if team_name.contains("East Carolina") {
        return "east".to_string();
    }
    if team_name.contains("Central Florida") || team_name.contains("UCF") {
        return "ucf".to_string();
    }
    if team_name.contains("Louisiana-Monroe") || team_name.contains("UL Monroe") {
        return "ul".to_string();
    }
    if team_name.contains("Texas-San Antonio") || team_name.contains("UTSA") {
        return "utsa".to_string();
    }

    let normalized = normalize_team_name(team_name);

    // Split by underscore
    let parts: Vec<&str> = normalized.split('_').collect();

    if parts.len() >= 2 {
        // Check for "X State" or "X St" patterns (where X can be multiple words)
        // Find if "st" appears in the parts (state gets converted to st by normalize_team_name)
        let state_index = parts.iter().position(|&p| p == "st");

        if let Some(idx) = state_index {
            // Include everything up to and including "st"
            // e.g., "san_diego_st" for San Diego State
            parts[..=idx].join("_")
        } else if parts.len() >= 2 && parts[1] == "dame" {
            // Handle "Notre Dame"
            format!("{}_{}", parts[0], parts[1])
        } else if parts.len() >= 2 && parts[1] == "aandm" {
            // Handle "Texas A&M" -> "texas_aandm"
            format!("{}_{}", parts[0], parts[1])
        } else if parts.len() >= 2
            && (parts[1] == "forest"
                || parts[1] == "texas"
                || parts[1] == "force"
                || parts[1] == "mexico"
                || parts[1] == "kentucky"
                || parts[1] == "virginia"
                || parts[1] == "michigan")
        {
            // Handle two-word schools: Wake Forest, North Texas, Air Force, New Mexico,
            // Western Kentucky, West Virginia, Western Michigan, etc.
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
    games_with_odds: &Vec<(Game, Vec<BettingOdds>)>,
    predictions: &Vec<GamePrediction>,
    top_n: usize,
) -> Result<Vec<EvBetRecommendation>> {
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
    let mut no_prediction_count = 0;
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
                    "No prediction found for: {} vs {} (odds api key: {})",
                    game.home_team, game.away_team, game_key
                );
                no_prediction_count += 1;
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
                        home_team: game.home_team.clone(),
                        away_team: game.away_team.clone(),
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

    println!(
        "Number of games without predictions: {}",
        no_prediction_count
    );

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
    pub home_team: String,
    pub away_team: String,
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

/// A spread bet recommendation with EV analysis
#[derive(Debug, Clone)]
pub struct SpreadEvBetRecommendation {
    pub home_team: String,
    pub away_team: String,
    pub team: String,
    pub spread_line: f64,
    pub bookmaker: String,
    pub odds: i32,
    pub model_spread: f64,
    pub model_prob: f64,
    pub implied_prob: f64,
    pub expected_value: f64,
    pub edge: f64,
}

impl SpreadEvBetRecommendation {
    /// Format the spread bet recommendation as a readable string
    pub fn format(&self) -> String {
        format!(
            "{} @ {} | Bet: {} ({:+.1}) ({:+}) on {} | EV: {:+.2}% | Edge: {:+.2}% | Model Spread: {:+.1} | Model: {:.1}% | Implied: {:.1}%",
            self.away_team,
            self.home_team,
            self.team,
            self.spread_line,
            self.odds,
            self.bookmaker,
            self.expected_value * 100.0,
            self.edge * 100.0,
            self.model_spread,
            self.model_prob * 100.0,
            self.implied_prob * 100.0
        )
    }
}

/// Analyze all available games and return the top N spread EV bets
pub async fn find_top_spread_ev_bets(
    games_with_odds: &Vec<(Game, Vec<BettingOdds>)>,
    game_predictions: &Vec<GamePrediction>,
    top_n: usize,
) -> Result<Vec<SpreadEvBetRecommendation>> {
    // Standard deviation for college football score predictions (typically 10-14 points)
    const STD_DEV: f64 = 12.0;

    // Create a lookup map for game predictions
    let mut prediction_map: HashMap<String, &GamePrediction> = HashMap::new();
    for pred in game_predictions {
        let home_key = extract_school_name(&pred.home_team);
        let away_key = extract_school_name(&pred.away_team);

        let game_key = format!("{}_{}", home_key, away_key);
        prediction_map.insert(game_key.clone(), pred);

        // Also store reverse key
        let reverse_key = format!("{}_{}", away_key, home_key);
        prediction_map.insert(reverse_key, pred);
    }

    // Calculate EV for all spread bets
    let mut all_bets = Vec::new();
    let mut no_prediction_count = 0;

    for (game, odds_list) in games_with_odds {
        // Extract school names from full team names
        let home_key = extract_school_name(&game.home_team);
        let away_key = extract_school_name(&game.away_team);

        // Try to find matching prediction
        let game_key = format!("{}_{}", home_key, away_key);
        let game_pred = match prediction_map.get(&game_key) {
            Some(pred) => pred,
            None => {
                no_prediction_count += 1;
                continue;
            }
        };

        // The prediction tracker spread is positive if the home team is predicted to win
        let model_spread = game_pred.spread;

        // Analyze each bookmaker's spread odds
        for bookmaker_odds in odds_list {
            for spread_odds in &bookmaker_odds.spreads {
                let team_key = extract_school_name(&spread_odds.team);
                let is_home_team = team_key == home_key;

                // The model_spread is from the home team's perspective (positive = home wins by that much)
                // The spread_odds.point is from the team's perspective in the bet and using normal betting lines
                // such as negative = spread_odds.team wins
                let cover_prob = if is_home_team {
                    // Betting on home team: use spread as-is
                    calculate_spread_cover_probability(model_spread, spread_odds.point, STD_DEV)
                } else {
                    // Betting on away team: we need the OPPOSITE condition
                    // If away has +12.5, they cover when home_margin < 12.5
                    calculate_spread_cover_probability(-model_spread, spread_odds.point, STD_DEV)
                };

                let implied_prob = american_odds_to_probability(spread_odds.price);
                let ev = calculate_expected_value(cover_prob, spread_odds.price);
                let edge = cover_prob - implied_prob;

                all_bets.push(SpreadEvBetRecommendation {
                    home_team: game.home_team.clone(),
                    away_team: game.away_team.clone(),
                    team: spread_odds.team.clone(),
                    spread_line: spread_odds.point,
                    bookmaker: bookmaker_odds.bookmaker.clone(),
                    odds: spread_odds.price,
                    model_spread,
                    model_prob: cover_prob,
                    implied_prob,
                    expected_value: ev,
                    edge,
                });
            }
        }
    }

    println!(
        "Spread bets - Number of games without predictions: {}",
        no_prediction_count
    );

    // Sort by EV (descending) and take top N
    all_bets.sort_by(|a, b| {
        b.expected_value
            .partial_cmp(&a.expected_value)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(all_bets.into_iter().take(top_n).collect())
}
