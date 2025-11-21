use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    pub price: i32, // American odds format (e.g., -110, +150)
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
    pub home_win_prob: f64, // Probability between 0 and 1
    pub away_win_prob: f64, // Probability between 0 and 1
}
