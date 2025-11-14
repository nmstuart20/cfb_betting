use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Represents a college football game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub home_team: String,
    pub away_team: String,
    pub commence_time: DateTime<Utc>,
    pub sport_title: String,
}

/// Moneyline odds for a team
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoneylineOdds {
    pub team: String,
    pub price: i32,  // American odds format (e.g., -110, +150)
}

/// Betting odds from a sportsbook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BettingOdds {
    pub game_id: String,
    pub bookmaker: String,
    pub last_update: DateTime<Utc>,
    pub moneyline: Vec<MoneylineOdds>,
}

/// Prediction from a model (ESPN FPI, Sagarin, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub game_id: String,
    pub model_name: String,
    pub home_team: String,
    pub away_team: String,
    pub home_win_prob: f64,  // Probability between 0 and 1
    pub away_win_prob: f64,  // Probability between 0 and 1
}

/// Expected value calculation for a bet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedValue {
    pub game_id: String,
    pub team: String,
    pub model_name: String,
    pub odds: i32,  // American odds
    pub implied_prob: f64,  // Implied probability from odds
    pub model_prob: f64,  // Probability from predictive model
    pub expected_value: f64,  // EV as a percentage
    pub edge: f64,  // Difference between model prob and implied prob
}

/// Combined game data with odds and predictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameAnalysis {
    pub game: Game,
    pub odds: Vec<BettingOdds>,
    pub predictions: Vec<Prediction>,
    pub expected_values: Vec<ExpectedValue>,
}
