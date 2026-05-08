use chrono::{DateTime, Utc};

use crate::market::{AlertItem, MarketRow, MarketSnapshot, TapeTrade};
use crate::news::{NewsItem, NewsSnapshot, Severity};

pub enum AppEvent {
    Market(MarketSnapshot),
    News(NewsSnapshot),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Symbol,
    Move,
    Funding,
    OpenInterest,
    Spread,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Timeframe {
    M1,
    M5,
    M15,
    H1,
}

impl Timeframe {
    pub fn label(self) -> &'static str {
        match self {
            Timeframe::M1 => "1m",
            Timeframe::M5 => "5m",
            Timeframe::M15 => "15m",
            Timeframe::H1 => "60m",
        }
    }

    pub fn sample_stride(self) -> usize {
        match self {
            Timeframe::M1 => 1,
            Timeframe::M5 => 3,
            Timeframe::M15 => 6,
            Timeframe::H1 => 12,
        }
    }
}

pub struct App {
    pub markets: Vec<MarketRow>,
    pub selected_symbol: usize,
    pub sort_mode: SortMode,
    pub timeframe: Timeframe,
    pub news_items: Vec<NewsItem>,
    pub selected_news: usize,
    pub active_symbol_only: bool,
    pub severe_only: bool,
    pub alerts: Vec<AlertItem>,
    pub last_market_update: Option<DateTime<Utc>>,
    pub last_news_update: Option<DateTime<Utc>>,
    pub news_status: String,
    pub market_status: String,
}

impl App {
    pub fn new() -> Self {
        Self {
            markets: Vec::new(),
            selected_symbol: 0,
            sort_mode: SortMode::Symbol,
            timeframe: Timeframe::M1,
            news_items: Vec::new(),
            selected_news: 0,
            active_symbol_only: true,
            severe_only: false,
            alerts: Vec::new(),
            last_market_update: None,
            last_news_update: None,
            news_status: "Booting RSS feeds...".to_string(),
            market_status: "Booting market monitor...".to_string(),
        }
    }

    pub fn apply_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Market(snapshot) => {
                let old_symbol = self.selected_market().map(|market| market.symbol.clone());
                self.alerts = snapshot.alerts;
                self.markets = self.sort_markets(snapshot.rows);
                self.last_market_update = Some(snapshot.updated_at);
                self.market_status = snapshot.status;
                self.selected_symbol = self.restore_selected_symbol(old_symbol);
            }
            AppEvent::News(snapshot) => {
                self.news_items = snapshot.items;
                self.last_news_update = Some(snapshot.updated_at);
                self.news_status = snapshot.status;
                self.selected_news = self
                    .selected_news
                    .min(self.filtered_news().len().saturating_sub(1));
            }
        }
    }

    pub fn select_prev_symbol(&mut self) {
        if self.markets.is_empty() {
            return;
        }
        self.selected_symbol = self.selected_symbol.saturating_sub(1);
        self.selected_news = 0;
    }

    pub fn select_next_symbol(&mut self) {
        if self.markets.is_empty() {
            return;
        }
        self.selected_symbol = (self.selected_symbol + 1).min(self.markets.len().saturating_sub(1));
        self.selected_news = 0;
    }

    pub fn prev_news(&mut self) {
        self.selected_news = self.selected_news.saturating_sub(1);
    }

    pub fn next_news(&mut self) {
        let max_index = self.filtered_news().len().saturating_sub(1);
        self.selected_news = (self.selected_news + 1).min(max_index);
    }

    pub fn toggle_active_symbol_filter(&mut self) {
        self.active_symbol_only = !self.active_symbol_only;
        self.selected_news = 0;
    }

    pub fn toggle_severity_filter(&mut self) {
        self.severe_only = !self.severe_only;
        self.selected_news = 0;
    }

    pub fn cycle_sort_mode(&mut self, mode: SortMode) {
        self.sort_mode = mode;
        let old_symbol = self.selected_market().map(|market| market.symbol.clone());
        self.markets = self.sort_markets(self.markets.clone());
        self.selected_symbol = self.restore_selected_symbol(old_symbol);
    }

    pub fn set_timeframe(&mut self, timeframe: Timeframe) {
        self.timeframe = timeframe;
    }

    pub fn selected_market(&self) -> Option<&MarketRow> {
        self.markets.get(self.selected_symbol)
    }

    pub fn selected_alerts(&self) -> Vec<&AlertItem> {
        self.alerts
            .iter()
            .filter(|alert| {
                self.selected_market()
                    .is_none_or(|market| alert.symbol == market.symbol || alert.symbol == "MARKET")
            })
            .collect()
    }

    pub fn selected_tape(&self) -> Vec<&TapeTrade> {
        self.selected_market()
            .map(|market| market.tape.iter().collect())
            .unwrap_or_default()
    }

    pub fn filtered_news(&self) -> Vec<&NewsItem> {
        let symbol = self.selected_market().map(|m| m.base_symbol.as_str());
        let filtered = self
            .news_items
            .iter()
            .filter(|item| {
                let symbol_ok = if self.active_symbol_only {
                    symbol.is_none_or(|sym| item.tags.iter().any(|tag| tag == sym))
                } else {
                    true
                };

                let severity_ok = if self.severe_only {
                    item.severity != Severity::Info
                } else {
                    true
                };

                symbol_ok && severity_ok
            })
            .collect::<Vec<_>>();

        if filtered.is_empty() && self.active_symbol_only {
            self.news_items
                .iter()
                .filter(|item| {
                    if self.severe_only {
                        item.severity != Severity::Info
                    } else {
                        true
                    }
                })
                .collect()
        } else {
            filtered
        }
    }

    fn sort_markets(&self, mut rows: Vec<MarketRow>) -> Vec<MarketRow> {
        match self.sort_mode {
            SortMode::Symbol => rows.sort_by(|a, b| a.symbol.cmp(&b.symbol)),
            SortMode::Move => rows.sort_by(|a, b| b.change_pct.total_cmp(&a.change_pct)),
            SortMode::Funding => rows.sort_by(|a, b| b.funding_bps.total_cmp(&a.funding_bps)),
            SortMode::OpenInterest => {
                rows.sort_by(|a, b| b.open_interest_m.total_cmp(&a.open_interest_m))
            }
            SortMode::Spread => rows.sort_by(|a, b| {
                let a_spread = a.best_ask - a.best_bid;
                let b_spread = b.best_ask - b.best_bid;
                b_spread.total_cmp(&a_spread)
            }),
        }
        rows
    }

    fn restore_selected_symbol(&self, old_symbol: Option<String>) -> usize {
        old_symbol
            .and_then(|symbol| {
                self.markets
                    .iter()
                    .position(|market| market.symbol == symbol)
            })
            .unwrap_or(0)
            .min(self.markets.len().saturating_sub(1))
    }
}
