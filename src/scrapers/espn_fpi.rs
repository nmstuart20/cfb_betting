use crate::models::Prediction;
use anyhow::{Result, Context};
use scraper::{Html, Selector};

const ESPN_FPI_URL: &str = "https://www.espn.com/college-football/fpi";

pub struct EspnFpiScraper {
    client: reqwest::Client,
}

impl EspnFpiScraper {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .build()
                .unwrap(),
        }
    }

    /// Scrape FPI predictions for upcoming games
    pub async fn fetch_predictions(&self) -> Result<Vec<Prediction>> {
        let html = self.client
            .get(ESPN_FPI_URL)
            .send()
            .await
            .context("Failed to fetch ESPN FPI page")?
            .text()
            .await?;

        self.parse_fpi_html(&html)
    }

    fn parse_fpi_html(&self, html: &str) -> Result<Vec<Prediction>> {
        let document = Html::parse_document(html);
        let mut predictions = Vec::new();

        // Note: This is a simplified parser. ESPN's actual FPI page structure
        // may require more sophisticated parsing, potentially including:
        // - Handling dynamic JavaScript content
        // - Parsing JSON embedded in the page
        // - Using ESPN's API if available

        // For now, we'll create a placeholder structure
        // You'll need to inspect ESPN's actual page structure and update this

        // Example selector (this will need to be updated based on ESPN's actual HTML)
        let game_selector = Selector::parse(".matchup").ok().context("Invalid selector")?;
        let team_selector = Selector::parse(".team-name").ok().context("Invalid selector")?;
        let prob_selector = Selector::parse(".win-prob").ok().context("Invalid selector")?;

        for game_elem in document.select(&game_selector) {
            // This is placeholder logic - actual implementation depends on ESPN's HTML structure
            let teams: Vec<String> = game_elem
                .select(&team_selector)
                .map(|el| el.text().collect::<String>().trim().to_string())
                .collect();

            let probs: Vec<f64> = game_elem
                .select(&prob_selector)
                .filter_map(|el| {
                    let text = el.text().collect::<String>();
                    text.trim().trim_end_matches('%').parse::<f64>().ok()
                        .map(|p| p / 100.0)
                })
                .collect();

            if teams.len() >= 2 && probs.len() >= 2 {
                let game_id = format!("{}_{}", teams[0], teams[1]);

                predictions.push(Prediction {
                    game_id: game_id.clone(),
                    model_name: "ESPN FPI".to_string(),
                    home_team: teams[0].clone(),
                    away_team: teams[1].clone(),
                    home_win_prob: probs[0],
                    away_win_prob: probs[1],
                });
            }
        }

        Ok(predictions)
    }

    /// Alternative: Fetch FPI data from ESPN's API if available
    /// This method would be preferred over scraping if ESPN provides an API
    pub async fn fetch_from_api(&self) -> Result<Vec<Prediction>> {
        // ESPN may have an internal API that powers their FPI page
        // This would require:
        // 1. Inspecting network requests on the FPI page
        // 2. Finding the API endpoint
        // 3. Reverse engineering the API format

        // For now, this is a placeholder
        anyhow::bail!("ESPN FPI API not yet implemented. Use fetch_predictions() for scraping.")
    }
}

impl Default for EspnFpiScraper {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to normalize team names
/// ESPN and betting sites may use different team names
pub fn normalize_team_name(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .replace("state", "st")
        .replace("&", "and")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_team_name() {
        assert_eq!(normalize_team_name("Ohio State"), "ohio st");
        assert_eq!(normalize_team_name("Texas A&M"), "texas aandm");
    }

    #[tokio::test]
    #[ignore]  // Ignore by default since it requires network access
    async fn test_fetch_predictions() {
        let scraper = EspnFpiScraper::new();
        let result = scraper.fetch_predictions().await;
        // This test will likely fail without proper selectors
        // Update the selectors based on ESPN's actual HTML structure
    }
}
