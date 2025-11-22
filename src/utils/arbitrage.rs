use crate::models::{BettingOdds, Game};
use crate::utils::ev_calculator::american_odds_to_probability;
use anyhow::Result;

/// Represents an arbitrage opportunity for a moneyline bet
#[derive(Debug, Clone)]
pub struct MoneylineArbitrage {
    pub home_team: String,
    pub away_team: String,
    pub home_bookmaker: String,
    pub away_bookmaker: String,
    pub home_odds: i32,
    pub away_odds: i32,
    pub profit_percentage: f64,
    pub home_stake_percentage: f64,
    pub away_stake_percentage: f64,
}

impl MoneylineArbitrage {
    pub fn format(&self) -> String {
        format!(
            "{} @ {} | Home: {} ({:+}) on {} [{:.2}%] | Away: {} ({:+}) on {} [{:.2}%] | Profit: {:.2}%",
            self.away_team,
            self.home_team,
            self.home_team,
            self.home_odds,
            self.home_bookmaker,
            self.home_stake_percentage,
            self.away_team,
            self.away_odds,
            self.away_bookmaker,
            self.away_stake_percentage,
            self.profit_percentage
        )
    }
}

/// Represents an arbitrage opportunity for a spread bet
#[derive(Debug, Clone)]
pub struct SpreadArbitrage {
    pub home_team: String,
    pub away_team: String,
    pub side1_team: String,
    pub side1_spread: f64,
    pub side1_odds: i32,
    pub side1_bookmaker: String,
    pub side2_team: String,
    pub side2_spread: f64,
    pub side2_odds: i32,
    pub side2_bookmaker: String,
    pub profit_percentage: f64,
    pub side1_stake_percentage: f64,
    pub side2_stake_percentage: f64,
}

impl SpreadArbitrage {
    pub fn format(&self) -> String {
        format!(
            "{} @ {} | {} ({:+.1}) ({:+}) on {} [{:.2}%] | {} ({:+.1}) ({:+}) on {} [{:.2}%] | Profit: {:.2}%",
            self.away_team,
            self.home_team,
            self.side1_team,
            self.side1_spread,
            self.side1_odds,
            self.side1_bookmaker,
            self.side1_stake_percentage,
            self.side2_team,
            self.side2_spread,
            self.side2_odds,
            self.side2_bookmaker,
            self.side2_stake_percentage,
            self.profit_percentage
        )
    }
}

/// Find arbitrage opportunities in moneyline bets
pub fn find_moneyline_arbitrage(
    games_with_odds: &[(Game, Vec<BettingOdds>)],
) -> Result<Vec<MoneylineArbitrage>> {
    let mut arbitrage_opportunities = Vec::new();

    for (game, odds_list) in games_with_odds {
        // Find best odds for home team across all bookmakers
        let mut best_home_odds: Option<(i32, String)> = None;
        let mut best_away_odds: Option<(i32, String)> = None;

        for bookmaker_odds in odds_list {
            for moneyline in &bookmaker_odds.moneyline {
                if moneyline.team == game.home_team {
                    if best_home_odds.is_none()
                        || moneyline.price > best_home_odds.as_ref().unwrap().0
                    {
                        best_home_odds = Some((moneyline.price, bookmaker_odds.bookmaker.clone()));
                    }
                } else if moneyline.team == game.away_team {
                    if best_away_odds.is_none()
                        || moneyline.price > best_away_odds.as_ref().unwrap().0
                    {
                        best_away_odds = Some((moneyline.price, bookmaker_odds.bookmaker.clone()));
                    }
                }
            }
        }

        if let (Some((home_odds, home_bookmaker)), Some((away_odds, away_bookmaker))) =
            (best_home_odds, best_away_odds)
        {
            // Calculate implied probabilities
            let home_prob = american_odds_to_probability(home_odds);
            let away_prob = american_odds_to_probability(away_odds);

            // Total implied probability
            let total_prob = home_prob + away_prob;

            // If total probability < 1, we have an arbitrage opportunity
            if total_prob < 1.0 {
                let profit_percentage = (1.0 / total_prob - 1.0) * 100.0;

                // Calculate optimal stake percentages
                let home_stake_percentage = (home_prob / total_prob) * 100.0;
                let away_stake_percentage = (away_prob / total_prob) * 100.0;

                arbitrage_opportunities.push(MoneylineArbitrage {
                    home_team: game.home_team.clone(),
                    away_team: game.away_team.clone(),
                    home_bookmaker,
                    away_bookmaker,
                    home_odds,
                    away_odds,
                    profit_percentage,
                    home_stake_percentage,
                    away_stake_percentage,
                });
            }
        }
    }

    // Sort by profit percentage (descending)
    arbitrage_opportunities.sort_by(|a, b| {
        b.profit_percentage
            .partial_cmp(&a.profit_percentage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(arbitrage_opportunities)
}

/// Find arbitrage opportunities in spread bets
pub fn find_spread_arbitrage(
    games_with_odds: &[(Game, Vec<BettingOdds>)],
) -> Result<Vec<SpreadArbitrage>> {
    let mut arbitrage_opportunities = Vec::new();

    for (game, odds_list) in games_with_odds {
        // Collect all spread odds for this game
        let mut all_spreads: Vec<(String, f64, i32, String)> = Vec::new();

        for bookmaker_odds in odds_list {
            for spread in &bookmaker_odds.spreads {
                all_spreads.push((
                    spread.team.clone(),
                    spread.point,
                    spread.price,
                    bookmaker_odds.bookmaker.clone(),
                ));
            }
        }

        // Look for arbitrage between opposing spreads
        for i in 0..all_spreads.len() {
            for j in (i + 1)..all_spreads.len() {
                let (team1, spread1, odds1, book1) = &all_spreads[i];
                let (team2, spread2, odds2, book2) = &all_spreads[j];

                // Check if these are opposing bets (one on each team)
                // and the spreads are equal and opposite (or close enough)
                if team1 != team2 && (spread1 + spread2).abs() < 0.1 {
                    let prob1 = american_odds_to_probability(*odds1);
                    let prob2 = american_odds_to_probability(*odds2);

                    let total_prob = prob1 + prob2;

                    // If total probability < 1, we have an arbitrage opportunity
                    if total_prob < 1.0 {
                        let profit_percentage = (1.0 / total_prob - 1.0) * 100.0;

                        // Calculate optimal stake percentages
                        let stake1_percentage = (prob1 / total_prob) * 100.0;
                        let stake2_percentage = (prob2 / total_prob) * 100.0;

                        arbitrage_opportunities.push(SpreadArbitrage {
                            home_team: game.home_team.clone(),
                            away_team: game.away_team.clone(),
                            side1_team: team1.clone(),
                            side1_spread: *spread1,
                            side1_odds: *odds1,
                            side1_bookmaker: book1.clone(),
                            side2_team: team2.clone(),
                            side2_spread: *spread2,
                            side2_odds: *odds2,
                            side2_bookmaker: book2.clone(),
                            profit_percentage,
                            side1_stake_percentage: stake1_percentage,
                            side2_stake_percentage: stake2_percentage,
                        });
                    }
                }
            }
        }
    }

    // Sort by profit percentage (descending)
    arbitrage_opportunities.sort_by(|a, b| {
        b.profit_percentage
            .partial_cmp(&a.profit_percentage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Remove duplicates (same arb from different perspectives)
    let mut seen = std::collections::HashSet::new();
    arbitrage_opportunities.retain(|arb| {
        let key = format!(
            "{}_{}_{}_{}_{}",
            arb.home_team,
            arb.away_team,
            arb.side1_bookmaker,
            arb.side2_bookmaker,
            arb.profit_percentage
        );
        seen.insert(key)
    });

    Ok(arbitrage_opportunities)
}
