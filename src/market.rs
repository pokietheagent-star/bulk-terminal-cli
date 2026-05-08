use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use bulk_client::api::parts::config::WSConfig;
use bulk_client::api::{BulkWsClient, Event, Topic};
use bulk_client::msgs::account::Fill;
use bulk_client::msgs::md::{L2Snapshot, Ticker};
use chrono::{DateTime, Utc};
use rand::Rng;

use crate::app::AppEvent;

pub struct MarketSnapshot {
    pub rows: Vec<MarketRow>,
    pub alerts: Vec<AlertItem>,
    pub updated_at: DateTime<Utc>,
    pub status: String,
}

#[derive(Clone, Default)]
pub struct OrderBookView {
    pub best_bid: f64,
    pub best_ask: f64,
    pub bid_depth_k: f64,
    pub ask_depth_k: f64,
    pub bids: Vec<OrderLevelView>,
    pub asks: Vec<OrderLevelView>,
}

#[derive(Clone)]
pub struct OrderLevelView {
    pub price: f64,
    pub size: f64,
}

#[derive(Clone)]
pub struct TapeTrade {
    pub time_label: String,
    pub side_label: String,
    pub price: f64,
    pub size: f64,
    pub is_buy: bool,
}

#[derive(Clone)]
pub struct AlertItem {
    pub symbol: String,
    pub severity: AlertSeverity,
    pub label: String,
    pub detail: String,
    pub time_label: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Clone)]
pub struct MarketRow {
    pub symbol: String,
    pub base_symbol: String,
    pub last_price: f64,
    pub mark_price: f64,
    pub funding_bps: f64,
    pub open_interest_m: f64,
    pub volume_24h_m: f64,
    pub change_pct: f64,
    pub best_bid: f64,
    pub best_ask: f64,
    pub bid_depth_k: f64,
    pub ask_depth_k: f64,
    pub oracle_gap_bps: f64,
    pub sparkline: Vec<u64>,
    pub price_history: Vec<f64>,
    pub funding_history: Vec<f64>,
    pub oi_history: Vec<f64>,
    pub gap_history: Vec<f64>,
    pub book: OrderBookView,
    pub tape: Vec<TapeTrade>,
}

struct MarketSeed {
    symbol: &'static str,
    base_symbol: &'static str,
    price: f64,
    funding_bps: f64,
    oi_m: f64,
    vol_m: f64,
}

pub struct MarketEngine {
    rows: HashMap<String, MarketRow>,
}

impl MarketEngine {
    pub fn new() -> Self {
        let seeds = [
            MarketSeed {
                symbol: "BTC-USD",
                base_symbol: "BTC",
                price: 84250.0,
                funding_bps: 1.8,
                oi_m: 96.0,
                vol_m: 240.0,
            },
            MarketSeed {
                symbol: "ETH-USD",
                base_symbol: "ETH",
                price: 4120.0,
                funding_bps: 2.2,
                oi_m: 61.0,
                vol_m: 182.0,
            },
            MarketSeed {
                symbol: "SOL-USD",
                base_symbol: "SOL",
                price: 188.0,
                funding_bps: 4.6,
                oi_m: 34.0,
                vol_m: 129.0,
            },
        ];

        let rows = seeds
            .into_iter()
            .map(|seed| {
                let mut sparkline = vec![50; 64];
                let row = MarketRow {
                    symbol: seed.symbol.to_string(),
                    base_symbol: seed.base_symbol.to_string(),
                    last_price: seed.price,
                    mark_price: seed.price * 0.9995,
                    funding_bps: seed.funding_bps,
                    open_interest_m: seed.oi_m,
                    volume_24h_m: seed.vol_m,
                    change_pct: 0.0,
                    best_bid: seed.price * 0.9997,
                    best_ask: seed.price * 1.0003,
                    bid_depth_k: seed.price * 1.7,
                    ask_depth_k: seed.price * 1.45,
                    oracle_gap_bps: 1.2,
                    sparkline: {
                        sparkline.push(55);
                        sparkline
                    },
                    price_history: vec![seed.price; 48],
                    funding_history: vec![seed.funding_bps; 48],
                    oi_history: vec![seed.oi_m; 48],
                    gap_history: vec![1.2; 48],
                    book: simulated_book(seed.price),
                    tape: vec![
                        TapeTrade {
                            time_label: "boot".to_string(),
                            side_label: "BUY".to_string(),
                            price: seed.price,
                            size: 0.20,
                            is_buy: true,
                        },
                        TapeTrade {
                            time_label: "boot".to_string(),
                            side_label: "SELL".to_string(),
                            price: seed.price * 0.999,
                            size: 0.12,
                            is_buy: false,
                        },
                    ],
                };
                (row.symbol.clone(), row)
            })
            .collect();

        Self { rows }
    }

    pub fn tick(&mut self) -> MarketSnapshot {
        let mut rng = rand::rng();

        for row in self.rows.values_mut() {
            let drift = match row.base_symbol.as_str() {
                "BTC" => rng.random_range(-0.004..0.004),
                "ETH" => rng.random_range(-0.006..0.006),
                _ => rng.random_range(-0.009..0.009),
            };

            row.last_price *= 1.0 + drift;
            row.mark_price = row.last_price * (1.0 + rng.random_range(-0.0008..0.0008));
            row.change_pct = (row.change_pct * 0.88) + (drift * 100.0 * 3.4);
            row.funding_bps = (row.funding_bps + rng.random_range(-0.35..0.35)).clamp(-12.0, 12.0);
            row.open_interest_m =
                (row.open_interest_m * (1.0 + rng.random_range(-0.015..0.015))).max(1.0);
            row.volume_24h_m = (row.volume_24h_m * (1.0 + rng.random_range(-0.02..0.03))).max(1.0);

            let spread = row.last_price * rng.random_range(0.00025..0.0009);
            row.best_bid = row.last_price - (spread / 2.0);
            row.best_ask = row.last_price + (spread / 2.0);
            row.bid_depth_k = (row.bid_depth_k * (1.0 + rng.random_range(-0.06..0.08))).max(10.0);
            row.ask_depth_k = (row.ask_depth_k * (1.0 + rng.random_range(-0.06..0.08))).max(10.0);
            row.oracle_gap_bps = rng.random_range(-6.0..6.0);
            row.book = simulated_book(row.last_price);
            row.price_history.push(row.last_price);
            row.funding_history.push(row.funding_bps);
            row.oi_history.push(row.open_interest_m);
            row.gap_history.push(row.oracle_gap_bps);
            if row.price_history.len() > 96 {
                row.price_history.remove(0);
            }
            if row.funding_history.len() > 96 {
                row.funding_history.remove(0);
            }
            if row.oi_history.len() > 96 {
                row.oi_history.remove(0);
            }
            if row.gap_history.len() > 96 {
                row.gap_history.remove(0);
            }
            row.tape.insert(
                0,
                TapeTrade {
                    time_label: Utc::now().format("%H:%M:%S").to_string(),
                    side_label: if drift >= 0.0 { "BUY" } else { "SELL" }.to_string(),
                    price: row.last_price,
                    size: rng.random_range(0.05..0.75),
                    is_buy: drift >= 0.0,
                },
            );
            row.tape.truncate(18);

            let spark_value = ((row.change_pct + 8.0) * 7.0).clamp(5.0, 100.0) as u64;
            row.sparkline.push(spark_value);
            if row.sparkline.len() > 96 {
                row.sparkline.remove(0);
            }
        }

        let mut rows = self.rows.values().cloned().collect::<Vec<_>>();
        rows.sort_by(|a, b| a.symbol.cmp(&b.symbol));

        MarketSnapshot {
            alerts: build_alerts(&rows, &HashMap::new(), Utc::now()),
            rows,
            updated_at: Utc::now(),
            status: "Live market simulation".to_string(),
        }
    }
}

pub fn spawn_market_thread(tx: Sender<AppEvent>) {
    thread::spawn(move || {
        if let Err(error) = run_bulk_market_loop(tx.clone()) {
            let _ = tx.send(AppEvent::Market(MarketSnapshot {
                rows: Vec::new(),
                alerts: vec![AlertItem {
                    symbol: "MARKET".to_string(),
                    severity: AlertSeverity::Warning,
                    label: "feed-fallback".to_string(),
                    detail: format!("{error}"),
                    time_label: Utc::now().format("%H:%M:%S").to_string(),
                }],
                updated_at: Utc::now(),
                status: format!("BULK feed unavailable: {error}. Falling back to simulation"),
            }));
            run_simulation_loop(tx);
        }
    });
}

fn run_simulation_loop(tx: Sender<AppEvent>) {
    let mut engine = MarketEngine::new();
    loop {
        let snapshot = engine.tick();
        if tx.send(AppEvent::Market(snapshot)).is_err() {
            break;
        }
        thread::sleep(Duration::from_millis(900));
    }
}

fn run_bulk_market_loop(tx: Sender<AppEvent>) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async move {
        let symbols = vec![
            "BTC-USD".to_string(),
            "ETH-USD".to_string(),
            "SOL-USD".to_string(),
        ];

        let client = BulkWsClient::connect(WSConfig {
            symbols: symbols.clone(),
            signer: None,
            track_account: false,
            ..Default::default()
        })
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;

        let books: Arc<Mutex<HashMap<String, OrderBookView>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let books_for_handler = Arc::clone(&books);
        let trades: Arc<Mutex<HashMap<String, Vec<TapeTrade>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let trades_for_handler = Arc::clone(&trades);

        client
            .on(Topic::L2Snapshot, move |event| {
                if let Event::L2Snapshot(book) = event {
                    let view = order_book_view(book);
                    if let Ok(mut guard) = books_for_handler.lock() {
                        guard.insert(book.symbol.clone(), view);
                    }
                }
            })
            .await;

        client
            .on(Topic::Trades, move |event| {
                if let Event::Trades(batch) = event {
                    if let Ok(mut guard) = trades_for_handler.lock() {
                        for fill in batch {
                            let entry = guard.entry(fill.symbol.clone()).or_default();
                            entry.insert(0, tape_from_fill(fill));
                            entry.truncate(18);
                        }
                    }
                }
            })
            .await;

        for symbol in &symbols {
            client
                .subscribe_l2_snapshot(symbol, Some(12))
                .await
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
            client
                .subscribe_trades(&[symbol.as_str()])
                .await
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        }

        let mut spark_state: HashMap<String, Vec<u64>> = symbols
            .iter()
            .map(|symbol| (symbol.clone(), vec![50; 64]))
            .collect();
        let mut price_history_state: HashMap<String, Vec<f64>> = symbols
            .iter()
            .map(|symbol| (symbol.clone(), Vec::new()))
            .collect();
        let mut funding_history_state: HashMap<String, Vec<f64>> = symbols
            .iter()
            .map(|symbol| (symbol.clone(), Vec::new()))
            .collect();
        let mut oi_history_state: HashMap<String, Vec<f64>> = symbols
            .iter()
            .map(|symbol| (symbol.clone(), Vec::new()))
            .collect();
        let mut gap_history_state: HashMap<String, Vec<f64>> = symbols
            .iter()
            .map(|symbol| (symbol.clone(), Vec::new()))
            .collect();
        let mut previous_rows: HashMap<String, MarketRow> = HashMap::new();

        loop {
            if !client.is_connected() {
                let _ = tx.send(AppEvent::Market(MarketSnapshot {
                    rows: Vec::new(),
                    alerts: vec![AlertItem {
                        symbol: "MARKET".to_string(),
                        severity: AlertSeverity::Critical,
                        label: "feed-disconnected".to_string(),
                        detail: "BULK websocket disconnected".to_string(),
                        time_label: Utc::now().format("%H:%M:%S").to_string(),
                    }],
                    updated_at: Utc::now(),
                    status: "BULK websocket disconnected".to_string(),
                }));
                break;
            }

            let tickers = client.get_tickers();
            let book_map = books.lock().map(|guard| guard.clone()).unwrap_or_default();
            let trade_map = trades.lock().map(|guard| guard.clone()).unwrap_or_default();

            if !tickers.is_empty() {
                let snapshot = build_bulk_snapshot(
                    tickers,
                    &book_map,
                    &trade_map,
                    &mut spark_state,
                    &mut price_history_state,
                    &mut funding_history_state,
                    &mut oi_history_state,
                    &mut gap_history_state,
                    &mut previous_rows,
                );
                if tx.send(AppEvent::Market(snapshot)).is_err() {
                    break;
                }
            }

            tokio::time::sleep(Duration::from_millis(700)).await;
        }

        client.shutdown().await;
        Ok(())
    })
}

fn build_bulk_snapshot(
    tickers: HashMap<String, Ticker>,
    books: &HashMap<String, OrderBookView>,
    trade_map: &HashMap<String, Vec<TapeTrade>>,
    spark_state: &mut HashMap<String, Vec<u64>>,
    price_history_state: &mut HashMap<String, Vec<f64>>,
    funding_history_state: &mut HashMap<String, Vec<f64>>,
    oi_history_state: &mut HashMap<String, Vec<f64>>,
    gap_history_state: &mut HashMap<String, Vec<f64>>,
    previous_rows: &mut HashMap<String, MarketRow>,
) -> MarketSnapshot {
    let now = Utc::now();

    let mut rows = tickers
        .into_values()
        .map(|ticker| {
            let book = books.get(&ticker.symbol).cloned().unwrap_or_default();
            let tape = trade_map.get(&ticker.symbol).cloned().unwrap_or_default();
            let spark = spark_state
                .entry(ticker.symbol.clone())
                .or_insert_with(|| vec![50; 64]);
            let price_history = price_history_state
                .entry(ticker.symbol.clone())
                .or_insert_with(Vec::new);
            let funding_history = funding_history_state
                .entry(ticker.symbol.clone())
                .or_insert_with(Vec::new);
            let oi_history = oi_history_state
                .entry(ticker.symbol.clone())
                .or_insert_with(Vec::new);
            let gap_history = gap_history_state
                .entry(ticker.symbol.clone())
                .or_insert_with(Vec::new);
            let spark_value = ((ticker.price_change_percent + 8.0) * 7.0).clamp(5.0, 100.0) as u64;
            spark.push(spark_value);
            if spark.len() > 96 {
                spark.remove(0);
            }
            price_history.push(ticker.last_price);
            if price_history.len() > 96 {
                price_history.remove(0);
            }

            let oracle_gap_bps = if ticker.oracle_price.is_finite() && ticker.oracle_price != 0.0 {
                ((ticker.mark_price - ticker.oracle_price) / ticker.oracle_price) * 10_000.0
            } else {
                0.0
            };
            funding_history.push(ticker.funding_rate * 10_000.0);
            oi_history.push(ticker.open_interest / 1_000_000.0);
            gap_history.push(oracle_gap_bps);
            if funding_history.len() > 96 {
                funding_history.remove(0);
            }
            if oi_history.len() > 96 {
                oi_history.remove(0);
            }
            if gap_history.len() > 96 {
                gap_history.remove(0);
            }

            MarketRow {
                base_symbol: ticker
                    .symbol
                    .split('-')
                    .next()
                    .unwrap_or(&ticker.symbol)
                    .to_string(),
                symbol: ticker.symbol,
                last_price: ticker.last_price,
                mark_price: ticker.mark_price,
                funding_bps: ticker.funding_rate * 10_000.0,
                open_interest_m: ticker.open_interest / 1_000_000.0,
                volume_24h_m: ticker.quote_volume / 1_000_000.0,
                change_pct: ticker.price_change_percent,
                best_bid: book.best_bid,
                best_ask: book.best_ask,
                bid_depth_k: book.bid_depth_k,
                ask_depth_k: book.ask_depth_k,
                oracle_gap_bps,
                sparkline: spark.clone(),
                price_history: price_history.clone(),
                funding_history: funding_history.clone(),
                oi_history: oi_history.clone(),
                gap_history: gap_history.clone(),
                book,
                tape,
            }
        })
        .collect::<Vec<_>>();

    rows.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    let alerts = build_alerts(&rows, previous_rows, now);
    previous_rows.clear();
    for row in &rows {
        previous_rows.insert(row.symbol.clone(), row.clone());
    }

    MarketSnapshot {
        rows,
        alerts,
        updated_at: now,
        status: "Live BULK websocket".to_string(),
    }
}

fn order_book_view(book: &L2Snapshot) -> OrderBookView {
    let (bids, asks) = &book.levels;
    OrderBookView {
        best_bid: bids.first().map(|level| level.price).unwrap_or_default(),
        best_ask: asks.first().map(|level| level.price).unwrap_or_default(),
        bid_depth_k: bids
            .iter()
            .map(|level| level.price * level.size)
            .sum::<f64>()
            / 1_000.0,
        ask_depth_k: asks
            .iter()
            .map(|level| level.price * level.size)
            .sum::<f64>()
            / 1_000.0,
        bids: bids
            .iter()
            .take(10)
            .map(|level| OrderLevelView {
                price: level.price,
                size: level.size,
            })
            .collect(),
        asks: asks
            .iter()
            .take(10)
            .map(|level| OrderLevelView {
                price: level.price,
                size: level.size,
            })
            .collect(),
    }
}

fn tape_from_fill(fill: &Fill) -> TapeTrade {
    let side_repr = format!("{:?}", fill.side).to_lowercase();
    let is_buy = side_repr.contains("buy");
    TapeTrade {
        time_label: format_timestamp(fill.timestamp),
        side_label: if is_buy { "BUY" } else { "SELL" }.to_string(),
        price: fill.price,
        size: fill.size,
        is_buy,
    }
}

fn format_timestamp(timestamp: u64) -> String {
    let seconds = (timestamp / 1_000_000_000) as i64;
    chrono::DateTime::<Utc>::from_timestamp(seconds, 0)
        .map(|time| time.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| Utc::now().format("%H:%M:%S").to_string())
}

fn build_alerts(
    rows: &[MarketRow],
    previous_rows: &HashMap<String, MarketRow>,
    now: DateTime<Utc>,
) -> Vec<AlertItem> {
    let mut alerts = Vec::new();

    for row in rows {
        let time_label = now.format("%H:%M:%S").to_string();
        let spread_bps = if row.last_price > 0.0 {
            ((row.best_ask - row.best_bid) / row.last_price) * 10_000.0
        } else {
            0.0
        };
        let depth_ratio = row.bid_depth_k / row.ask_depth_k.max(1.0);

        if row.funding_bps.abs() >= 8.0 {
            alerts.push(AlertItem {
                symbol: row.symbol.clone(),
                severity: AlertSeverity::Warning,
                label: "funding-hot".to_string(),
                detail: format!("funding {:.2} bps", row.funding_bps),
                time_label: time_label.clone(),
            });
        }

        if spread_bps >= 8.0 {
            alerts.push(AlertItem {
                symbol: row.symbol.clone(),
                severity: AlertSeverity::Warning,
                label: "spread-wide".to_string(),
                detail: format!("spread {:.2} bps", spread_bps),
                time_label: time_label.clone(),
            });
        }

        if depth_ratio >= 1.35 || depth_ratio <= 0.74 {
            alerts.push(AlertItem {
                symbol: row.symbol.clone(),
                severity: AlertSeverity::Info,
                label: "depth-imbalance".to_string(),
                detail: format!("ratio {:.2}", depth_ratio),
                time_label: time_label.clone(),
            });
        }

        if row.oracle_gap_bps.abs() >= 12.0 {
            alerts.push(AlertItem {
                symbol: row.symbol.clone(),
                severity: AlertSeverity::Critical,
                label: "oracle-gap".to_string(),
                detail: format!("gap {:.2} bps", row.oracle_gap_bps),
                time_label: time_label.clone(),
            });
        }

        if let Some(previous) = previous_rows.get(&row.symbol) {
            if (row.change_pct - previous.change_pct).abs() >= 1.2 {
                alerts.push(AlertItem {
                    symbol: row.symbol.clone(),
                    severity: AlertSeverity::Info,
                    label: "fast-move".to_string(),
                    detail: format!(
                        "24h change moved from {:+.2}% to {:+.2}%",
                        previous.change_pct, row.change_pct
                    ),
                    time_label: time_label.clone(),
                });
            }

            let oi_delta = row.open_interest_m - previous.open_interest_m;
            if oi_delta.abs() >= 1.5 {
                alerts.push(AlertItem {
                    symbol: row.symbol.clone(),
                    severity: AlertSeverity::Info,
                    label: "oi-jump".to_string(),
                    detail: format!("OI delta {:+.2}M", oi_delta),
                    time_label: time_label.clone(),
                });
            }
        }
    }

    alerts.sort_by(|a, b| b.time_label.cmp(&a.time_label));
    alerts.truncate(16);
    alerts
}

fn simulated_book(price: f64) -> OrderBookView {
    let bids = (0..10)
        .map(|idx| OrderLevelView {
            price: price - (idx as f64 * price * 0.0003),
            size: 0.5 + (idx as f64 * 0.15),
        })
        .collect::<Vec<_>>();
    let asks = (0..10)
        .map(|idx| OrderLevelView {
            price: price + (idx as f64 * price * 0.0003),
            size: 0.45 + (idx as f64 * 0.13),
        })
        .collect::<Vec<_>>();

    OrderBookView {
        best_bid: bids.first().map(|level| level.price).unwrap_or_default(),
        best_ask: asks.first().map(|level| level.price).unwrap_or_default(),
        bid_depth_k: bids
            .iter()
            .map(|level| level.price * level.size)
            .sum::<f64>()
            / 1_000.0,
        ask_depth_k: asks
            .iter()
            .map(|level| level.price * level.size)
            .sum::<f64>()
            / 1_000.0,
        bids,
        asks,
    }
}
