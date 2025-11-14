use crate::models::Prediction;
use anyhow::{Context, Result};
use scraper::{Html, Selector};
use std::collections::HashMap;

const SAGARIN_URL: &str = "https://sagarin.com/sports/cfsend.htm";

#[derive(Debug, Clone)]
pub struct SagarinRating {
    pub team: String,
    pub rating: f64,
    pub rank: usize,
}

pub struct SagarinScraper {
    client: reqwest::Client,
}

impl SagarinScraper {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap(),
        }
    }

    /// Fetch Sagarin ratings for all teams
    pub async fn fetch_ratings(&self) -> Result<Vec<SagarinRating>> {
        let html = self
            .client
            .get(SAGARIN_URL)
            .send()
            .await
            .context("Failed to fetch Sagarin ratings")?
            .text()
            .await?;

        self.parse_ratings_html(&html)
    }

    fn parse_ratings_html(&self, html: &str) -> Result<Vec<SagarinRating>> {
        let document = Html::parse_document(html);
        let mut ratings = Vec::new();

        // Sagarin's page has a specific format - typically a <pre> tag with plain text
        // The format is usually:
        // RANK TEAM                 RATING
        //    1 Georgia              95.50
        //    2 Michigan             94.20
        // etc.

        let pre_selector = Selector::parse("pre").ok().context("Invalid selector")?;

        for pre_elem in document.select(&pre_selector) {
            let text = pre_elem.text().collect::<String>();

            // Parse each line
            for line in text.lines() {
                if let Some(rating) = self.parse_rating_line(line) {
                    ratings.push(rating);
                }
            }

            // Break after first <pre> tag that contains ratings
            if !ratings.is_empty() {
                break;
            }
        }

        Ok(ratings)
    }

    fn parse_rating_line(&self, line: &str) -> Option<SagarinRating> {
        // Example line: "   1 Georgia              95.50"
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 3 {
            return None;
        }

        let rank = parts[0].parse::<usize>().ok()?;
        let rating = parts.last()?.parse::<f64>().ok()?;

        // Team name is everything between rank and rating
        let team = parts[1..parts.len() - 1].join(" ");

        Some(SagarinRating { team, rating, rank })
    }

    /// Calculate win probability between two teams based on rating difference
    /// Uses a logistic regression formula commonly used in college football
    pub fn calculate_win_probability(
        &self,
        team_rating: f64,
        opponent_rating: f64,
        home_advantage: f64,
    ) -> f64 {
        // Sagarin ratings can be converted to win probability using the formula:
        // Win% = 1 / (1 + 10^((opponent_rating - team_rating - home_advantage) / spread_factor))

        const SPREAD_FACTOR: f64 = 25.0; // Typical value for college football

        let rating_diff = team_rating - opponent_rating + home_advantage;
        let exponent = -rating_diff / SPREAD_FACTOR;

        1.0 / (1.0 + 10_f64.powf(exponent))
    }

    /// Generate predictions for a list of matchups
    pub async fn generate_predictions(
        &self,
        matchups: Vec<(String, String, String)>, // (game_id, home_team, away_team)
    ) -> Result<Vec<Prediction>> {
        let ratings = self.fetch_ratings().await?;
        let rating_map: HashMap<String, f64> = ratings
            .iter()
            .map(|r| (r.team.to_lowercase(), r.rating))
            .collect();

        const HOME_ADVANTAGE: f64 = 3.0; // Typical home field advantage in points

        let mut predictions = Vec::new();

        for (game_id, home_team, away_team) in matchups {
            let home_rating = rating_map.get(&home_team.to_lowercase());
            let away_rating = rating_map.get(&away_team.to_lowercase());

            if let (Some(&home_r), Some(&away_r)) = (home_rating, away_rating) {
                let home_win_prob = self.calculate_win_probability(home_r, away_r, HOME_ADVANTAGE);
                let away_win_prob = 1.0 - home_win_prob;

                predictions.push(Prediction {
                    game_id,
                    model_name: "Sagarin".to_string(),
                    home_team,
                    away_team,
                    home_win_prob,
                    away_win_prob,
                });
            }
        }

        Ok(predictions)
    }
}

impl Default for SagarinScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rating_line() {
        let scraper = SagarinScraper::new();

        let line = "   1 Georgia              95.50";
        let rating = scraper.parse_rating_line(line).unwrap();
        assert_eq!(rating.rank, 1);
        assert_eq!(rating.team, "Georgia");
        assert!((rating.rating - 95.50).abs() < 0.01);
    }

    #[test]
    fn test_calculate_win_probability() {
        let scraper = SagarinScraper::new();

        // Equal teams, no home advantage
        let prob = scraper.calculate_win_probability(90.0, 90.0, 0.0);
        assert!((prob - 0.5).abs() < 0.01);

        // Home team much stronger
        let prob = scraper.calculate_win_probability(95.0, 85.0, 3.0);
        assert!(prob > 0.7);

        // Away team stronger despite home advantage
        let prob = scraper.calculate_win_probability(85.0, 95.0, 3.0);
        assert!(prob < 0.5);
    }

    #[tokio::test]
    async fn test_fetch_ratings() {
        let scraper = SagarinScraper::new();
        let ratings = scraper.fetch_ratings().await.unwrap();
        assert!(!ratings.is_empty());
        println!("Fetched {} team ratings", ratings.len());
    }
}
