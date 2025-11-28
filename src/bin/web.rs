use askama::Template;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use cfb_betting_ev::fetch_all_betting_data;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

// Custom filters for formatting
mod filters {
    use chrono::{DateTime, Utc};

    pub fn format_odds(odds: &i32) -> ::askama::Result<String> {
        Ok(format!("{:+}", odds))
    }

    pub fn format_percent(value: &f64) -> ::askama::Result<String> {
        Ok(format!("{:.1}%", value * 100.0))
    }

    pub fn format_arb_percent(value: &f64) -> ::askama::Result<String> {
        Ok(format!("{:.2}%", value))
    }

    pub fn format_spread(value: &f64) -> ::askama::Result<String> {
        Ok(format!("{:+.1}", value))
    }

    pub fn format_money(value: &f64) -> ::askama::Result<String> {
        Ok(format!("{:.2}", value))
    }

    pub fn calc_profit(profit_pct: &f64) -> ::askama::Result<String> {
        let profit = (profit_pct / 100.0) * 100.0;
        Ok(format!("{:.2}", profit))
    }

    pub fn date(s: &str) -> ::askama::Result<String> {
        let dt = s.parse::<DateTime<Utc>>().unwrap();
        Ok(dt.format("%Y-%m-%d").to_string())
    }
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    active_page: String,
    cfb_moneyline_count: usize,
    cfb_spread_count: usize,
    cfb_arb_count: usize,
    cbb_arb_count: usize,
    cfb_game_results_count: usize,
    cbb_game_results_count: usize,
    show_top_bets: bool,
    top_bets: Vec<cfb_betting_ev::utils::ev_analysis::EvBetRecommendation>,
}

#[derive(Template)]
#[template(path = "cfb.html")]
struct CfbTemplate {
    active_page: String,
    cfb_moneyline_arbs: Vec<cfb_betting_ev::utils::arbitrage::MoneylineArbitrage>,
    cfb_spread_arbs: Vec<cfb_betting_ev::utils::arbitrage::SpreadArbitrage>,
}

#[derive(Template)]
#[template(path = "cfb_moneyline.html")]
struct CfbMoneylineTemplate {
    active_page: String,
    cfb_moneyline_bets: Vec<cfb_betting_ev::utils::ev_analysis::EvBetRecommendation>,
    cfb_moneyline_arbs: Vec<cfb_betting_ev::utils::arbitrage::MoneylineArbitrage>,
}

#[derive(Template)]
#[template(path = "cfb_spread.html")]
struct CfbSpreadTemplate {
    active_page: String,
    cfb_spread_bets: Vec<cfb_betting_ev::utils::ev_analysis::SpreadEvBetRecommendation>,
    cfb_spread_arbs: Vec<cfb_betting_ev::utils::arbitrage::SpreadArbitrage>,
}

#[derive(Template)]
#[template(path = "cbb.html")]
struct CbbTemplate {
    active_page: String,
    cbb_moneyline_arbs: Vec<cfb_betting_ev::utils::arbitrage::MoneylineArbitrage>,
    cbb_spread_arbs: Vec<cfb_betting_ev::utils::arbitrage::SpreadArbitrage>,
}

#[derive(Template)]
#[template(path = "cfb_results.html")]
struct CfbResultsTemplate {
    active_page: String,
    cfb_game_results: Vec<cfb_betting_ev::api::game_results_api::GameResult>,
}

#[derive(Template)]
#[template(path = "cbb_results.html")]
struct CbbResultsTemplate {
    active_page: String,
    cbb_game_results: Vec<cfb_betting_ev::api::game_results_api::CbbGameResult>,
}

#[derive(Template)]
#[template(path = "cfb_bet_results.html")]
struct CfbBetResultsTemplate {
    active_page: String,
    cfb_moneyline_bet_results: Vec<cfb_betting_ev::utils::ev_analysis::BetResult>,
    cfb_spread_bet_results: Vec<cfb_betting_ev::utils::ev_analysis::SpreadBetResult>,
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

// Shared state to cache data
type SharedData = Arc<RwLock<Option<cfb_betting_ev::BettingData>>>;

async fn home(data: axum::extract::State<SharedData>) -> impl IntoResponse {
    let betting_data = data.read().await;

    let data = match betting_data.as_ref() {
        Some(d) => d.clone(),
        None => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Data not loaded yet").into_response();
        }
    };

    let cfb_moneyline_count = data.cfb_moneyline_bets.len();
    let cfb_spread_count = data.cfb_spread_bets.len();
    let cfb_arb_count = data.cfb_moneyline_arbs.len() + data.cfb_spread_arbs.len();
    let cbb_arb_count = data.cbb_moneyline_arbs.len() + data.cbb_spread_arbs.len();
    let cfb_game_results_count = data.cfb_game_results.len();
    let cbb_game_results_count = data.cbb_game_results.len();

    // Get top 3 bets
    let top_bets: Vec<_> = data.cfb_moneyline_bets.iter().take(3).cloned().collect();
    let show_top_bets = !top_bets.is_empty();

    let template = HomeTemplate {
        active_page: "home".to_string(),
        cfb_moneyline_count,
        cfb_spread_count,
        cfb_arb_count,
        cbb_arb_count,
        cfb_game_results_count,
        cbb_game_results_count,
        show_top_bets,
        top_bets,
    };

    HtmlTemplate(template).into_response()
}

async fn cfb(data: axum::extract::State<SharedData>) -> impl IntoResponse {
    let betting_data = data.read().await;

    let data = match betting_data.as_ref() {
        Some(d) => d.clone(),
        None => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Data not loaded yet").into_response();
        }
    };

    let template = CfbTemplate {
        active_page: "cfb".to_string(),
        cfb_moneyline_arbs: data.cfb_moneyline_arbs,
        cfb_spread_arbs: data.cfb_spread_arbs,
    };

    HtmlTemplate(template).into_response()
}

async fn cfb_moneyline(data: axum::extract::State<SharedData>) -> impl IntoResponse {
    let betting_data = data.read().await;

    let data = match betting_data.as_ref() {
        Some(d) => d.clone(),
        None => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Data not loaded yet").into_response();
        }
    };

    let template = CfbMoneylineTemplate {
        active_page: "cfb_moneyline".to_string(),
        cfb_moneyline_bets: data.cfb_moneyline_bets,
        cfb_moneyline_arbs: data.cfb_moneyline_arbs,
    };

    HtmlTemplate(template).into_response()
}

async fn cfb_spread(data: axum::extract::State<SharedData>) -> impl IntoResponse {
    let betting_data = data.read().await;

    let data = match betting_data.as_ref() {
        Some(d) => d.clone(),
        None => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Data not loaded yet").into_response();
        }
    };

    let template = CfbSpreadTemplate {
        active_page: "cfb_spread".to_string(),
        cfb_spread_bets: data.cfb_spread_bets,
        cfb_spread_arbs: data.cfb_spread_arbs,
    };

    HtmlTemplate(template).into_response()
}

async fn cbb(data: axum::extract::State<SharedData>) -> impl IntoResponse {
    let betting_data = data.read().await;

    let data = match betting_data.as_ref() {
        Some(d) => d.clone(),
        None => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Data not loaded yet").into_response();
        }
    };

    let template = CbbTemplate {
        active_page: "cbb".to_string(),
        cbb_moneyline_arbs: data.cbb_moneyline_arbs,
        cbb_spread_arbs: data.cbb_spread_arbs,
    };

    HtmlTemplate(template).into_response()
}

async fn cfb_results(data: axum::extract::State<SharedData>) -> impl IntoResponse {
    let betting_data = data.read().await;

    let data = match betting_data.as_ref() {
        Some(d) => d.clone(),
        None => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Data not loaded yet").into_response();
        }
    };

    let template = CfbResultsTemplate {
        active_page: "cfb_results".to_string(),
        cfb_game_results: data.cfb_game_results,
    };

    HtmlTemplate(template).into_response()
}

async fn cbb_results(data: axum::extract::State<SharedData>) -> impl IntoResponse {
    let betting_data = data.read().await;

    let data = match betting_data.as_ref() {
        Some(d) => d.clone(),
        None => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Data not loaded yet").into_response();
        }
    };

    let template = CbbResultsTemplate {
        active_page: "cbb_results".to_string(),
        cbb_game_results: data.cbb_game_results,
    };

    HtmlTemplate(template).into_response()
}

async fn cfb_bet_results(data: axum::extract::State<SharedData>) -> impl IntoResponse {
    let betting_data = data.read().await;

    let data = match betting_data.as_ref() {
        Some(d) => d.clone(),
        None => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Data not loaded yet").into_response();
        }
    };

    let template = CfbBetResultsTemplate {
        active_page: "cfb_bet_results".to_string(),
        cfb_moneyline_bet_results: data.cfb_moneyline_bet_results,
        cfb_spread_bet_results: data.cfb_spread_bet_results,
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

    let use_cache = std::env::var("USE_CACHE").unwrap_or_default() == "1";

    // Fetch data on startup
    let data = match fetch_all_betting_data(use_cache).await {
        Ok(data) => {
            println!("Data loaded successfully");
            println!(
                "  - {} CFB Moneyline EV Bets",
                data.cfb_moneyline_bets.len()
            );
            println!("  - {} CFB Spread EV Bets", data.cfb_spread_bets.len());
            println!(
                "  - {} CFB Arbitrage Opportunities",
                data.cfb_moneyline_arbs.len() + data.cfb_spread_arbs.len()
            );
            println!(
                "  - {} CBB Arbitrage Opportunities",
                data.cbb_moneyline_arbs.len() + data.cbb_spread_arbs.len()
            );
            println!("  - {} CFB Game Results", data.cfb_game_results.len());
            println!("  - {} CBB Game Results", data.cbb_game_results.len());
            Arc::new(RwLock::new(Some(data)))
        }
        Err(e) => {
            eprintln!("Error fetching data: {}", e);
            eprintln!("Server will start but pages may show errors");
            Arc::new(RwLock::new(None))
        }
    };

    println!("\nStarting web server at http://127.0.0.1:3000");
    println!("Press Ctrl+C to stop\n");

    // Build router with routes
    let app = Router::new()
        // This will serve files from the "static" directory at the "/static" URL path
        .nest_service("/static", ServeDir::new("static"))
        .route("/", get(home))
        .route("/cfb", get(cfb))
        .route("/cfb/moneyline", get(cfb_moneyline))
        .route("/cfb/spread", get(cfb_spread))
        .route("/cfb/results", get(cfb_results))
        .route("/cfb/bet-results", get(cfb_bet_results))
        .route("/cbb", get(cbb))
        .route("/cbb/results", get(cbb_results))
        .with_state(data);

    // Run server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
