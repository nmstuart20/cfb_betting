use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a college football or basketball game
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

/// Spread odds for a team
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpreadOdds {
    pub team: String,
    pub point: f64, // Spread points (e.g., -7.5, +3.5)
    pub price: i32, // American odds format (e.g., -110, +150)
}

/// Betting odds from a sportsbook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BettingOdds {
    pub game_id: String,
    pub bookmaker: String,
    pub last_update: DateTime<Utc>,
    pub moneyline: Vec<MoneylineOdds>,
    pub spreads: Vec<SpreadOdds>,
}
