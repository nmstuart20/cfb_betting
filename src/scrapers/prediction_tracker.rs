use anyhow::{Context, Result};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

const PREDICTION_TRACKER_URL: &str = "https://www.thepredictiontracker.com/predncaa.html";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GamePrediction {
    pub home_team: String,
    pub away_team: String,
    pub spread: f64,
    pub home_win_prob: f64,
    pub away_win_prob: f64,
    pub _prediction_avg: f64,
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

    /// Scrape game predictions (with spread data) from The Prediction Tracker
    pub async fn fetch_game_predictions(&self) -> Result<Vec<GamePrediction>> {
        let html = self
            .client
            .get(PREDICTION_TRACKER_URL)
            .send()
            .await
            .context("Failed to fetch Prediction Tracker page")?
            .text()
            .await?;

        self.parse_html_to_game_predictions(&html)
    }

    fn parse_html_to_game_predictions(&self, html: &str) -> Result<Vec<GamePrediction>> {
        let document = Html::parse_document(html);
        let mut game_predictions = Vec::new();

        // The Prediction Tracker uses plain text tables within <pre> tags
        let pre_selector = Selector::parse("pre")
            .ok()
            .context("Invalid pre selector")?;

        for pre_elem in document.select(&pre_selector) {
            let text = pre_elem.text().collect::<String>();

            // Parse the plain text table
            for line in text.lines() {
                if let Some(game) = self.parse_text_line(line) {
                    game_predictions.push(game);
                }
            }
        }

        Ok(game_predictions)
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

        // Split teams by looking for multiple consecutive spaces (2+ spaces)
        // This separates "Air Force" from "New Mexico" but keeps each team name intact
        let team_parts: Vec<&str> = teams_str
            .split("  ") // Split on 2+ spaces
            .filter(|s| !s.trim().is_empty())
            .collect();

        if team_parts.len() < 2 {
            return None;
        }

        let home_team = team_parts[0].trim().replace(".", "").to_string();
        let away_team = team_parts[1].trim().replace(".", "").to_string();
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
            .ok()?;

        // Validate that win probability is reasonable
        if !(0.0..=1.0).contains(&home_win_prob) {
            return None;
        }

        Some(GamePrediction {
            home_team,
            away_team,
            spread,
            home_win_prob, // Convert percentage to decimal
            away_win_prob: 1.0 - home_win_prob,
            _prediction_avg: prediction_avg,
        })
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
        let result = scraper.fetch_game_predictions().await;
        assert!(result.is_ok());

        if let Ok(predictions) = result {
            println!("Found {} predictions", predictions.len());
            for pred in predictions.iter().take(5) {
                println!(
                    "{} vs {} - Home win prob: {:.2}%, Away win prob: {:.2}%",
                    pred.home_team,
                    pred.away_team,
                    pred.home_win_prob * 100.0,
                    pred.away_win_prob * 100.0,
                );
            }
        }
    }
}
