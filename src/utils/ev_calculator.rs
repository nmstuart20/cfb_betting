/// Convert American odds to implied probability
/// Positive odds (+150) mean you win $150 on a $100 bet
/// Negative odds (-150) mean you need to bet $150 to win $100
pub fn american_odds_to_probability(odds: i32) -> f64 {
    if odds > 0 {
        // For positive odds: 100 / (odds + 100)
        100.0 / (odds as f64 + 100.0)
    } else {
        // For negative odds: |odds| / (|odds| + 100)
        let abs_odds = odds.abs() as f64;
        abs_odds / (abs_odds + 100.0)
    }
}

/// Convert probability to American odds
pub fn probability_to_american_odds(prob: f64) -> i32 {
    if prob >= 0.5 {
        // Favorite (negative odds)
        -((prob / (1.0 - prob)) * 100.0) as i32
    } else {
        // Underdog (positive odds)
        (((1.0 - prob) / prob) * 100.0) as i32
    }
}

/// Calculate expected value for a bet
/// EV = (probability of winning * amount won per bet) - (probability of losing * amount lost per bet)
/// Returns EV as a percentage of the bet amount
pub fn calculate_expected_value(model_prob: f64, odds: i32) -> f64 {
    let win_amount = if odds > 0 {
        odds as f64 / 100.0
    } else {
        100.0 / odds.abs() as f64
    };

    let lose_amount = 1.0; // You lose your bet amount
    let prob_lose = 1.0 - model_prob;

    // EV = (prob_win * win_amount) - (prob_lose * lose_amount)
    (model_prob * win_amount) - (prob_lose * lose_amount)
}

/// Calculate the edge (difference between model probability and implied probability)
pub fn calculate_edge(model_prob: f64, implied_prob: f64) -> f64 {
    model_prob - implied_prob
}

/// Determine if a bet has positive expected value
pub fn is_positive_ev(ev: f64) -> bool {
    ev > 0.0
}

/// Format EV as a percentage string
pub fn format_ev_percentage(ev: f64) -> String {
    format!("{:+.2}%", ev * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_american_odds_to_probability() {
        // Positive odds
        let prob = american_odds_to_probability(150);
        // Negative odds
        let prob = american_odds_to_probability(-150);
        assert!((prob - 0.6).abs() < 0.01);
        // Even odds
        let prob = american_odds_to_probability(100);
        assert!((prob - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_calculate_expected_value() {
        // Positive EV scenario: 60% win probability on +150 odds
        let ev = calculate_expected_value(0.6, 150);
        assert!(ev > 0.0);

        // Negative EV scenario: 40% win probability on -150 odds
        let ev = calculate_expected_value(0.4, -150);
        assert!(ev < 0.0);
    }

    #[test]
    fn test_calculate_edge() {
        let edge = calculate_edge(0.6, 0.5);
        assert!((edge - 0.1).abs() < 0.01);
    }
}
