use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use cfb_betting_ev::fetch_all_betting_data;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    cfb_moneyline_bets: Vec<cfb_betting_ev::utils::ev_analysis::EvBetRecommendation>,
    cfb_spread_bets: Vec<cfb_betting_ev::utils::ev_analysis::SpreadEvBetRecommendation>,
    cfb_moneyline_arbs: Vec<cfb_betting_ev::utils::arbitrage::MoneylineArbitrage>,
    cfb_spread_arbs: Vec<cfb_betting_ev::utils::arbitrage::SpreadArbitrage>,
    cbb_moneyline_arbs: Vec<cfb_betting_ev::utils::arbitrage::MoneylineArbitrage>,
    cbb_spread_arbs: Vec<cfb_betting_ev::utils::arbitrage::SpreadArbitrage>,
}

// Custom filters for formatting
mod filters {
    pub fn format_odds(odds: &i32) -> ::askama::Result<String> {
        Ok(format!("{:+}", odds))
    }

    pub fn format_percent(value: &f64) -> ::askama::Result<String> {
        Ok(format!("{:.1}%", value * 100.0))
    }

    pub fn format_arb_percent(value: &f64) -> ::askama::Result<String> {
        Ok(format!("{:.1}%", value))
    }

    pub fn format_spread(value: &f64) -> ::askama::Result<String> {
        Ok(format!("{:+.1}", value))
    }

    pub fn format_money(value: &f64) -> ::askama::Result<String> {
        Ok(format!("{:.2}", value))
    }
}

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template: {}", err),
            )
                .into_response(),
        }
    }
}

async fn index() -> impl IntoResponse {
    // Fetch betting data (use cache by default for web to avoid excessive API calls)
    let data = match fetch_all_betting_data(true).await {
        Ok(data) => data,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error fetching betting data: {}", e),
            )
                .into_response();
        }
    };

    let template = IndexTemplate {
        cfb_moneyline_bets: data.cfb_moneyline_bets,
        cfb_spread_bets: data.cfb_spread_bets,
        cfb_moneyline_arbs: data.cfb_moneyline_arbs,
        cfb_spread_arbs: data.cfb_spread_arbs,
        cbb_moneyline_arbs: data.cbb_moneyline_arbs,
        cbb_spread_arbs: data.cbb_spread_arbs,
    };

    HtmlTemplate(template).into_response()
}

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Fetching betting data...");
    println!("Starting web server at http://127.0.0.1:3000");
    println!("Press Ctrl+C to stop\n");

    // Build router
    let app = Router::new().route("/", get(index));

    // Run server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
