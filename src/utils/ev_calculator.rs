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
pub fn _probability_to_american_odds(prob: f64) -> i32 {
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

/// Calculate the probability of covering a spread
/// Uses a normal distribution approximation based on the predicted spread
///
/// model_spread: The predicted point differential (home team perspective, positive = home favored)
/// bet_spread: The betting line (e.g., -7.5 means home team must win by more than 7.5)
/// std_dev: Standard deviation of the prediction (typically 10-14 points for CFB)
pub fn calculate_spread_cover_probability(model_spread: f64, bet_spread: f64, std_dev: f64) -> f64 {
    // For a spread bet:
    // - bet_spread = -7 means the team is favored by 7, must win by MORE than 7 to cover
    // - bet_spread = +7 means the team is an underdog by 7, must not lose by MORE than 7 to cover
    //
    // The team covers if: actual_margin > bet_spread (in absolute terms)
    // For bet_spread = -7: team needs actual_margin > 7 (win by more than 7)
    // For bet_spread = +7: team needs actual_margin > -7 (lose by less than 7, or win)
    //
    // We model actual_margin ~ Normal(model_spread, std_dev)

    // The threshold is the absolute value when negative (favorite), or the value itself when positive
    let threshold = if bet_spread < 0.0 {
        bet_spread.abs() // Favorite: must win by more than this
    } else {
        -bet_spread // Underdog: must not lose by more than this (i.e., margin > -bet_spread)
    };

    // P(actual_margin > threshold) where actual_margin ~ Normal(model_spread, std_dev)
    // = P(Z > (threshold - model_spread) / std_dev)
    // = 1 - CDF((threshold - model_spread) / std_dev)
    let z = (threshold - model_spread) / std_dev;

    1.0 - normal_cdf(z)
}

/// Approximation of the standard normal cumulative distribution function
/// Using the error function approximation
fn normal_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

/// Approximation of the error function using Abramowitz and Stegun formula
fn erf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();

    let t = 1.0 / (1.0 + p * x);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

    sign * y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_american_odds_to_probability() {
        // Positive odds
        let prob = american_odds_to_probability(150);
        assert!((prob - 0.4).abs() < 0.01);
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
    fn test_calculate_spread_cover_probability() {
        // If model predicts home team wins by 10, and spread is -7, should have high probability
        let prob = calculate_spread_cover_probability(10.0, -7.0, 12.0);
        assert!(prob > 0.5);
        println!("Prob: {}", prob);

        // If model predicts home team wins by 3, and spread is -7, should have low probability
        let prob = calculate_spread_cover_probability(3.0, -7.0, 12.0);
        assert!(prob < 0.5);
        println!("Prob: {}", prob);

        // If model predicts home team wins by 5, and the spread is +5, very high probability (this should never happen)
        let prob = calculate_spread_cover_probability(5.0, 5.0, 12.0);
        assert!(prob > 0.7);
        println!("Prob: {}", prob);

        // If model predicts home team loses by 5, and the spread is +5, this should be close to 50%
        let prob = calculate_spread_cover_probability(-5.0, 5.0, 12.0);
        assert!((prob - 0.5).abs() < 0.1);
        println!("Prob: {}", prob);

        // Equal values should be close to 50%
        let prob = calculate_spread_cover_probability(7.0, -7.0, 12.0);
        assert!((prob - 0.5).abs() < 0.1);
        println!("Prob: {}", prob);

        // Equal values should be close to 50%
        let prob = calculate_spread_cover_probability(12.5, -12.5, 12.0);
        assert!((prob - 0.5).abs() < 0.1);
        println!("Prob: {}", prob);
    }
}
