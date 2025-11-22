# College Sports Betting EV & Arbitrage Calculator

A Rust-based tool for calculating expected value (EV) on college football betting opportunities and finding arbitrage opportunities across college football and basketball games.

## Features

- **Moneyline EV Analysis**: Identifies positive EV bets on CFB moneyline markets
- **Spread EV Analysis**: Calculates expected value for CFB point spread bets using normal distribution modeling
- **Arbitrage Detection**: Finds arbitrage opportunities across different sportsbooks for guaranteed profit
  - **College Football**: Moneyline and spread arbitrage
  - **College Basketball**: Moneyline and spread arbitrage
- **Multiple Sportsbooks**: Compares odds across major US sportsbooks via The Odds API
- **Predictive Models**: Uses consensus predictions from Prediction Tracker for CFB EV analysis
- **Odds Caching**: Cache odds data locally to avoid unnecessary API calls
- **CSV Export**: Export recommendations to CSV files for further analysis

## How It Works

The calculator:
1. Fetches current betting odds from The Odds API for college football and basketball
2. Scrapes predictive model data from Prediction Tracker (for CFB EV analysis)
3. Calculates expected value by comparing model probabilities against implied odds (CFB only)
4. Identifies arbitrage opportunities across all sportsbooks (CFB and CBB)
5. Ranks bets by EV and edge percentage
6. Optionally exports results to CSV files

### EV Calculation

**Moneyline**: Compares the model's win probability against the implied probability from American odds.

**Spreads**: Uses a normal distribution (σ = 12 points) to calculate the probability of covering the spread based on the predicted margin of victory.

### Arbitrage Detection

**How it works**: The program identifies opportunities where you can bet on all possible outcomes across different sportsbooks and guarantee a profit regardless of the result.

**Moneyline Arbitrage**: Finds cases where betting on both teams at different sportsbooks yields a profit.

**Spread Arbitrage**: Finds cases where taking opposite sides of a spread at different books guarantees profit.

**Calculation**: For each opportunity, the program calculates:
- Profit percentage (guaranteed return on investment)
- Optimal stake distribution (what percentage to bet on each side)

## Installation

### Prerequisites

- Rust (install from [rustup.rs](https://rustup.rs))
- The Odds API key (get one free at [the-odds-api.com](https://the-odds-api.com))

### Setup

1. Clone the repository:
```bash
git clone <repository-url>
cd cfb-betting-ev
```

2. Create a `.env` file in the project root:
```bash
echo "ODDS_API_KEY=your_api_key_here" > .env
```

3. Build the project:
```bash
cargo build --release
```

## Usage

The application can be run in two modes: **CLI mode** for terminal output and **Web mode** for a browser-based interface.

### Web Interface (Recommended)

Run the web server to view results in your browser:
```bash
cargo run --release --bin web
```

Then open your browser to `http://localhost:3000`

The web interface provides:
- Clean, organized display of all betting opportunities
- Color-coded cards for easy identification (EV bets in blue/gray, arbitrage in red/green)
- Responsive design for mobile and desktop
- Automatic caching to minimize API usage
- All data displayed on a single page

### CLI Mode

Run the command-line version with live data from APIs:
```bash
cargo run --release --bin cli
```

### Using Cached Data (CLI only)

To avoid consuming API credits, use cached data:
```bash
USE_CACHE=1 cargo run --release --bin cli
```

**Note**: The web interface automatically uses cached data by default.

Cache files are stored in `cache/`:
- `odds_cache.json` - Betting odds data
- `predictions_cache.json` - Model predictions

### Export to CSV (CLI only)

Save results to CSV files:
```bash
SAVE_CSV=1 cargo run --release --bin cli
```

This creates:
- `moneyline_bets.csv` - Top CFB moneyline EV bets
- `spread_bets.csv` - Top CFB spread EV bets
- `cfb_moneyline_arbitrage.csv` - CFB moneyline arbitrage opportunities (if any)
- `cfb_spread_arbitrage.csv` - CFB spread arbitrage opportunities (if any)
- `cbb_moneyline_arbitrage.csv` - CBB moneyline arbitrage opportunities (if any)
- `cbb_spread_arbitrage.csv` - CBB spread arbitrage opportunities (if any)

### Combined Options (CLI only)

Use cache and export to CSV:
```bash
USE_CACHE=1 SAVE_CSV=1 cargo run --release --bin cli
```

## Output Format

### Console Output

**EV Bets:**
```
=== MONEYLINE BETS ===

Top 30 Moneyline EV Bets:

1. Away Team @ Home Team | Bet: Team Name (+150) on Bookmaker | EV: +15.5% | Edge: +8.2% | Model: 45.0% | Implied: 36.8%
```

- **EV (Expected Value)**: Return per dollar wagered
- **Edge**: Difference between model probability and implied probability
- **Model**: Model's predicted win probability
- **Implied**: Bookmaker's implied probability from the odds

**Arbitrage Opportunities:**
```
=== ARBITRAGE OPPORTUNITIES ===

MONEYLINE ARBITRAGE

1. Away Team @ Home Team | Home: Home Team (+120) on BookmakerA [51.2%] | Away: Away Team (+125) on BookmakerB [48.8%] | Profit: 2.5%
```

- **Stake %**: Percentage of total bankroll to wager on each side
- **Profit %**: Guaranteed return regardless of outcome

### CSV Output

CSV files contain the same data in a spreadsheet-friendly format with headers for easy sorting and filtering.

## Project Structure

```
src/
├── main.rs                           # Entry point and CSV export
├── api/
│   └── odds_api.rs                   # The Odds API client
├── models/
│   └── mod.rs                        # Data structures
├── scrapers/
│   └── prediction_tracker.rs        # Prediction Tracker scraper
└── utils/
    ├── ev_calculator.rs              # EV and probability calculations
    └── ev_analysis.rs                # Bet analysis and matching
```

## API Usage

The program uses [The Odds API](https://the-odds-api.com) which offers:
- 500 free requests per month
- Coverage of major US sportsbooks
- Real-time odds updates

Each run typically uses 1-2 API requests depending on the number of games.

## Limitations

- Only analyzes FBS college football games
- Prediction Tracker may not cover all matchups (especially FCS opponents)
- Spread calculations assume a normal distribution with 12-point standard deviation
- Does not account for:
  - Juice/vig optimization
  - Bankroll management
  - Game-time injuries or lineup changes
  - Live betting opportunities

## Disclaimer

This tool is for educational and informational purposes only. Sports betting involves risk, and you should never bet more than you can afford to lose. Past performance of models does not guarantee future results. Always gamble responsibly.

## License

This project is provided as-is for educational purposes.

## Contributing

Contributions are welcome! Areas for improvement:
- Additional predictive models
- Over/under totals analysis
- Player props
- Alternate spread lines
- Kelly criterion bankroll management
- Historical performance tracking

## Acknowledgments

- [The Odds API](https://the-odds-api.com) for odds data
- [Prediction Tracker](https://www.thepredictiontracker.com) for model predictions
