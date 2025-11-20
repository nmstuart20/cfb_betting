use crate::models::Prediction;
use anyhow::{Context, Result};
use scraper::{Html, Selector};

const PREDICTION_TRACKER_URL: &str = "https://www.thepredictiontracker.com/predncaa.html";

#[derive(Debug, Clone)]
pub struct GamePrediction {
    pub home_team: String,
    pub away_team: String,
    pub spread: f64,
    pub home_win_prob: f64,
    pub prediction_avg: f64,
}

pub struct PredictionTrackerScraper {
    client: reqwest::Client,
}

impl PredictionTrackerScraper {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap(),
        }
    }

    /// Scrape predictions from The Prediction Tracker
    pub async fn fetch_predictions(&self) -> Result<Vec<Prediction>> {
        let html = self
            .client
            .get(PREDICTION_TRACKER_URL)
            .send()
            .await
            .context("Failed to fetch Prediction Tracker page")?
            .text()
            .await?;

        self.parse_html(&html)
    }

    fn parse_html(&self, html: &str) -> Result<Vec<Prediction>> {
        let document = Html::parse_document(html);
        let mut predictions = Vec::new();

        // The Prediction Tracker uses plain text tables within <pre> tags
        let pre_selector = Selector::parse("pre")
            .ok()
            .context("Invalid pre selector")?;

        for pre_elem in document.select(&pre_selector) {
            let text = pre_elem.text().collect::<String>();

            // Parse the plain text table
            for line in text.lines() {
                if let Some(game) = self.parse_text_line(line) {
                    let game_id = format!(
                        "{}_{}",
                        normalize_team_name(&game.home_team),
                        normalize_team_name(&game.away_team)
                    );

                    predictions.push(Prediction {
                        game_id,
                        model_name: "Prediction Tracker".to_string(),
                        home_team: game.home_team.clone(),
                        away_team: game.away_team.clone(),
                        home_win_prob: game.home_win_prob / 100.0, // Convert percentage to decimal
                        away_win_prob: (100.0 - game.home_win_prob) / 100.0,
                    });
                }
            }
        }

        Ok(predictions)
    }

    fn parse_text_line(&self, line: &str) -> Option<GamePrediction> {
        // Skip empty lines and header lines
        if line.trim().is_empty() || line.contains("Home") || line.contains("Visitor") {
            return None;
        }

        // The data uses fixed-width columns with significant spacing between team names and numbers
        // Format: "Home Team      Away Team         num1  num2  num3  ... prob1 prob2"

        // Find where the numeric data starts (first occurrence of a digit or negative sign followed by digit)
        let numeric_start = line
            .char_indices()
            .find(|(_, c)| c.is_ascii_digit())
            .map(|(i, _)| i)?;

        // Extract the team names portion (everything before the numbers)
        let teams_str = line[..numeric_start].trim_end();

        println!("Teams str: {}", teams_str);
        // Split teams by looking for multiple consecutive spaces (2+ spaces)
        // This separates "Air Force" from "New Mexico" but keeps each team name intact
        let team_parts: Vec<&str> = teams_str
            .split("  ") // Split on 2+ spaces
            .filter(|s| !s.trim().is_empty())
            .collect();
        println!("Team Parts: {:?}", team_parts);

        if team_parts.len() < 2 {
            return None;
        }

        let home_team = team_parts[0].trim().to_string();
        let away_team = team_parts[1].trim().to_string();

        // Extract numeric values
        let numeric_str = line[numeric_start..].trim();
        let numeric_parts: Vec<&str> = numeric_str.split_whitespace().collect();

        if numeric_parts.len() < 6 {
            return None;
        }

        // Parse numeric values
        // Expected columns: Opening, Updated, Midweek, Pred Avg, Pred Median, Std Dev, Min, Max, Prob Win, Prob Cover
        let spread = numeric_parts.get(1)?.parse::<f64>().ok()?; // Updated line
        let prediction_avg = numeric_parts.get(3)?.parse::<f64>().ok().unwrap_or(0.0);
        let home_win_prob = numeric_parts
            .get(numeric_parts.len() - 2)? // Second to last is win probability
            .parse::<f64>()
            .ok()?
            * 100.0; // Convert from decimal to percentage

        // Validate that win probability is reasonable
        if home_win_prob < 0.0 || home_win_prob > 100.0 {
            return None;
        }

        Some(GamePrediction {
            home_team,
            away_team,
            spread,
            home_win_prob,
            prediction_avg,
        })
    }

    /// Fetch and return raw game predictions with all data
    pub async fn fetch_game_predictions(&self) -> Result<Vec<GamePrediction>> {
        let html = self
            .client
            .get(PREDICTION_TRACKER_URL)
            .send()
            .await
            .context("Failed to fetch Prediction Tracker page")?
            .text()
            .await?;

        self.parse_games(&html)
    }

    fn parse_games(&self, html: &str) -> Result<Vec<GamePrediction>> {
        let document = Html::parse_document(html);
        let mut games = Vec::new();

        let pre_selector = Selector::parse("pre")
            .ok()
            .context("Invalid pre selector")?;

        for pre_elem in document.select(&pre_selector) {
            let text = pre_elem.text().collect::<String>();

            // Parse the plain text table
            for line in text.lines() {
                if let Some(game) = self.parse_text_line(line) {
                    games.push(game);
                }
            }
        }

        Ok(games)
    }
}

impl Default for PredictionTrackerScraper {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to normalize team names for consistent matching
pub fn normalize_team_name(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .replace("state", "st")
        .replace("&", "and")
        .replace(" ", "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_team_name() {
        assert_eq!(normalize_team_name("Ohio State"), "ohio_st");
        assert_eq!(normalize_team_name("Texas A&M"), "texas_aandm");
        assert_eq!(normalize_team_name("Florida Intl"), "florida_intl");
    }

    #[tokio::test]
    async fn test_fetch_predictions() {
        let scraper = PredictionTrackerScraper::new();
        let result = scraper.fetch_predictions().await;
        assert!(result.is_ok());

        if let Ok(predictions) = result {
            println!("Found {} predictions", predictions.len());
            for pred in predictions.iter().take(5) {
                println!(
                    "{} vs {} - Home win prob: {:.2}%",
                    pred.home_team,
                    pred.away_team,
                    pred.home_win_prob * 100.0
                );
            }
        }
    }

    #[tokio::test]
    async fn test_fetch_game_predictions() {
        let scraper = PredictionTrackerScraper::new();
        let result = scraper.fetch_game_predictions().await;
        assert!(result.is_ok());
        assert!(false);

        if let Ok(games) = result {
            println!("Found {} games", games.len());
            for game in games.iter().take(5) {
                println!(
                    "{} vs {} - Spread: {:.1}, Win prob: {:.2}%",
                    game.home_team, game.away_team, game.spread, game.home_win_prob
                );
            }
        }
    }
}
