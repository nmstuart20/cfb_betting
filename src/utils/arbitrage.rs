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
                } else if moneyline.team == game.away_team
                    && (best_away_odds.is_none()
                        || moneyline.price > best_away_odds.as_ref().unwrap().0)
                {
                    best_away_odds = Some((moneyline.price, bookmaker_odds.bookmaker.clone()));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{BettingOdds, Game, MoneylineOdds, SpreadOdds};
    use chrono::Utc;

    fn create_test_game(home: &str, away: &str) -> Game {
        Game {
            id: "test_game_1".to_string(),
            home_team: home.to_string(),
            away_team: away.to_string(),
            commence_time: Utc::now(),
            sport_title: "Test Sport".to_string(),
        }
    }

    fn create_betting_odds(
        game_id: &str,
        bookmaker: &str,
        moneyline: Vec<MoneylineOdds>,
        spreads: Vec<SpreadOdds>,
    ) -> BettingOdds {
        BettingOdds {
            game_id: game_id.to_string(),
            bookmaker: bookmaker.to_string(),
            last_update: Utc::now(),
            moneyline,
            spreads,
        }
    }

    #[test]
    fn test_moneyline_arbitrage_found() {
        // Setup: Create a game with arbitrage opportunity
        // BookA: Home +120 (45.5% implied), BookB: Away +125 (44.4% implied)
        // Total: 89.9% < 100%, so arbitrage exists
        let game = create_test_game("Home Team", "Away Team");

        let book_a_odds = create_betting_odds(
            &game.id,
            "BookmakerA",
            vec![MoneylineOdds {
                team: "Home Team".to_string(),
                price: 120,
            }],
            vec![],
        );

        let book_b_odds = create_betting_odds(
            &game.id,
            "BookmakerB",
            vec![MoneylineOdds {
                team: "Away Team".to_string(),
                price: 125,
            }],
            vec![],
        );

        let games_with_odds = vec![(game.clone(), vec![book_a_odds, book_b_odds])];

        let result = find_moneyline_arbitrage(&games_with_odds).unwrap();

        assert_eq!(result.len(), 1);
        let arb = &result[0];
        assert_eq!(arb.home_team, "Home Team");
        assert_eq!(arb.away_team, "Away Team");
        assert_eq!(arb.home_odds, 120);
        assert_eq!(arb.away_odds, 125);
        assert!(arb.profit_percentage > 0.0);
        assert!(arb.home_stake_percentage + arb.away_stake_percentage > 99.0);
        assert!(arb.home_stake_percentage + arb.away_stake_percentage < 101.0);
    }

    #[test]
    fn test_moneyline_no_arbitrage() {
        // Setup: No arbitrage opportunity (normal vig)
        // BookA: Home -110 (52.4% implied), Away -110 (52.4% implied)
        // Total: 104.8% > 100%, so no arbitrage
        let game = create_test_game("Home Team", "Away Team");

        let book_a_odds = create_betting_odds(
            &game.id,
            "BookmakerA",
            vec![
                MoneylineOdds {
                    team: "Home Team".to_string(),
                    price: -110,
                },
                MoneylineOdds {
                    team: "Away Team".to_string(),
                    price: -110,
                },
            ],
            vec![],
        );

        let games_with_odds = vec![(game.clone(), vec![book_a_odds])];

        let result = find_moneyline_arbitrage(&games_with_odds).unwrap();

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_moneyline_arbitrage_multiple_bookmakers() {
        // Setup: Multiple bookmakers, best odds create arbitrage
        let game = create_test_game("Home Team", "Away Team");

        let book_a_odds = create_betting_odds(
            &game.id,
            "BookmakerA",
            vec![
                MoneylineOdds {
                    team: "Home Team".to_string(),
                    price: 110, // Not the best
                },
                MoneylineOdds {
                    team: "Away Team".to_string(),
                    price: 105, // Not the best
                },
            ],
            vec![],
        );

        let book_b_odds = create_betting_odds(
            &game.id,
            "BookmakerB",
            vec![MoneylineOdds {
                team: "Home Team".to_string(),
                price: 130, // Best home odds
            }],
            vec![],
        );

        let book_c_odds = create_betting_odds(
            &game.id,
            "BookmakerC",
            vec![MoneylineOdds {
                team: "Away Team".to_string(),
                price: 140, // Best away odds
            }],
            vec![],
        );

        let games_with_odds = vec![(game.clone(), vec![book_a_odds, book_b_odds, book_c_odds])];

        let result = find_moneyline_arbitrage(&games_with_odds).unwrap();

        assert_eq!(result.len(), 1);
        let arb = &result[0];
        assert_eq!(arb.home_odds, 130); // Should pick best odds
        assert_eq!(arb.away_odds, 140); // Should pick best odds
        assert_eq!(arb.home_bookmaker, "BookmakerB");
        assert_eq!(arb.away_bookmaker, "BookmakerC");
    }

    #[test]
    fn test_spread_arbitrage_found() {
        // Setup: Spread arbitrage opportunity
        let game = create_test_game("Home Team", "Away Team");

        let book_a_odds = create_betting_odds(
            &game.id,
            "BookmakerA",
            vec![],
            vec![SpreadOdds {
                team: "Home Team".to_string(),
                point: -7.0,
                price: 110, // +110 offers arbitrage opportunity
            }],
        );

        let book_b_odds = create_betting_odds(
            &game.id,
            "BookmakerB",
            vec![],
            vec![SpreadOdds {
                team: "Away Team".to_string(),
                point: 7.0,
                price: 110, // +110 offers arbitrage opportunity
            }],
        );

        let games_with_odds = vec![(game.clone(), vec![book_a_odds, book_b_odds])];

        let result = find_spread_arbitrage(&games_with_odds).unwrap();

        assert_eq!(result.len(), 1);
        let arb = &result[0];
        assert_eq!(arb.side1_spread, -7.0);
        assert_eq!(arb.side2_spread, 7.0);
        assert!(arb.profit_percentage > 0.0);
    }

    #[test]
    fn test_spread_no_arbitrage() {
        // Setup: No spread arbitrage (normal vig)
        let game = create_test_game("Home Team", "Away Team");

        let book_a_odds = create_betting_odds(
            &game.id,
            "BookmakerA",
            vec![],
            vec![
                SpreadOdds {
                    team: "Home Team".to_string(),
                    point: -7.0,
                    price: -110,
                },
                SpreadOdds {
                    team: "Away Team".to_string(),
                    point: 7.0,
                    price: -110,
                },
            ],
        );

        let games_with_odds = vec![(game.clone(), vec![book_a_odds])];

        let result = find_spread_arbitrage(&games_with_odds).unwrap();

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_spread_arbitrage_ignores_non_matching_spreads() {
        // Setup: Spreads don't match up (different lines)
        let game = create_test_game("Home Team", "Away Team");

        let book_a_odds = create_betting_odds(
            &game.id,
            "BookmakerA",
            vec![],
            vec![SpreadOdds {
                team: "Home Team".to_string(),
                point: -7.0,
                price: 110,
            }],
        );

        let book_b_odds = create_betting_odds(
            &game.id,
            "BookmakerB",
            vec![],
            vec![SpreadOdds {
                team: "Away Team".to_string(),
                point: 6.5, // Doesn't match -7.0
                price: 110,
            }],
        );

        let games_with_odds = vec![(game.clone(), vec![book_a_odds, book_b_odds])];

        let result = find_spread_arbitrage(&games_with_odds).unwrap();

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_empty_games_returns_empty() {
        let games_with_odds: Vec<(Game, Vec<BettingOdds>)> = vec![];

        let moneyline_result = find_moneyline_arbitrage(&games_with_odds).unwrap();
        let spread_result = find_spread_arbitrage(&games_with_odds).unwrap();

        assert_eq!(moneyline_result.len(), 0);
        assert_eq!(spread_result.len(), 0);
    }

    #[test]
    fn test_arbitrage_profit_calculation() {
        // Test specific profit percentage calculation
        let game = create_test_game("Home Team", "Away Team");

        // Using specific odds to verify profit calculation
        // Home +100 (50% implied), Away +110 (47.6% implied)
        // Total: 97.6%, profit should be about 2.4%
        let book_a_odds = create_betting_odds(
            &game.id,
            "BookmakerA",
            vec![MoneylineOdds {
                team: "Home Team".to_string(),
                price: 100,
            }],
            vec![],
        );

        let book_b_odds = create_betting_odds(
            &game.id,
            "BookmakerB",
            vec![MoneylineOdds {
                team: "Away Team".to_string(),
                price: 110,
            }],
            vec![],
        );

        let games_with_odds = vec![(game.clone(), vec![book_a_odds, book_b_odds])];

        let result = find_moneyline_arbitrage(&games_with_odds).unwrap();

        assert_eq!(result.len(), 1);
        let arb = &result[0];

        // Profit should be approximately 2.4%
        assert!(arb.profit_percentage > 2.0);
        assert!(arb.profit_percentage < 3.0);
    }
}
