use std::collections::{HashMap, HashSet};
use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use rss::Channel;

pub struct NewsSnapshot {
    pub items: Vec<NewsItem>,
    pub updated_at: DateTime<Utc>,
    pub status: String,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Severity {
    Info,
    Medium,
    High,
}

#[derive(Clone)]
pub struct NewsItem {
    pub title: String,
    pub source: String,
    pub link: String,
    pub published_at: String,
    pub tags: Vec<String>,
    pub severity: Severity,
}

#[derive(Clone, Copy)]
struct FeedSource {
    name: &'static str,
    url: &'static str,
}

pub struct NewsPoller {
    client: Client,
    feeds: Vec<FeedSource>,
    seen: HashSet<String>,
    cache: Vec<NewsItem>,
}

impl Default for NewsPoller {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("bulk-monitor/0.1")
                .build()
                .expect("reqwest client"),
            feeds: vec![
                FeedSource {
                    name: "CoinDesk",
                    url: "https://www.coindesk.com/arc/outboundfeeds/rss/",
                },
                FeedSource {
                    name: "Cointelegraph",
                    url: "https://cointelegraph.com/rss.xml",
                },
            ],
            seen: HashSet::new(),
            cache: Vec::new(),
        }
    }
}

impl NewsPoller {
    pub fn poll(&mut self) -> NewsSnapshot {
        let mut merged = Vec::new();
        let mut healthy = 0usize;
        let mut errors = Vec::new();

        for feed in self.feeds.clone() {
            match self.fetch_feed(&feed) {
                Ok(mut items) => {
                    healthy += 1;
                    merged.append(&mut items);
                }
                Err(error) => errors.push(format!("{}: {}", feed.name, error)),
            }
        }

        if !merged.is_empty() {
            merged.sort_by(|a, b| b.published_at.cmp(&a.published_at));
            merged.truncate(120);
            self.cache = merged;
        }

        let status = if healthy == self.feeds.len() {
            format!("RSS live from {} free feeds", healthy)
        } else if healthy > 0 {
            format!(
                "RSS partial: {} ok, {} failed",
                healthy,
                self.feeds.len() - healthy
            )
        } else if !self.cache.is_empty() {
            "RSS unavailable, showing cached headlines".to_string()
        } else {
            format!("RSS unavailable: {}", errors.join(" | "))
        };

        NewsSnapshot {
            items: self.cache.clone(),
            updated_at: Utc::now(),
            status,
        }
    }

    fn fetch_feed(&mut self, feed: &FeedSource) -> anyhow::Result<Vec<NewsItem>> {
        let bytes = self
            .client
            .get(feed.url)
            .send()?
            .error_for_status()?
            .bytes()?;
        let channel = Channel::read_from(&bytes[..])?;
        let mut items = Vec::new();

        for item in channel.items().iter().take(30) {
            let title = item.title().unwrap_or("Untitled").trim().to_string();
            let link = item.link().unwrap_or_default().to_string();
            let normalized = normalize_title(&title);

            if normalized.is_empty() || self.seen.contains(&normalized) {
                continue;
            }

            self.seen.insert(normalized);

            let tags = detect_tags(&title);
            let severity = detect_severity(&title);
            let published_at = item
                .pub_date()
                .and_then(parse_pub_date)
                .unwrap_or_else(|| Utc::now().format("%Y-%m-%d %H:%M UTC").to_string());

            items.push(NewsItem {
                title,
                source: feed.name.to_string(),
                link,
                published_at,
                tags,
                severity,
            });
        }

        Ok(items)
    }
}

fn parse_pub_date(input: &str) -> Option<String> {
    DateTime::parse_from_rfc2822(input).ok().map(|date| {
        date.with_timezone(&Utc)
            .format("%Y-%m-%d %H:%M UTC")
            .to_string()
    })
}

fn normalize_title(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn detect_tags(title: &str) -> Vec<String> {
    let rules = HashMap::from([
        ("btc", "BTC"),
        ("bitcoin", "BTC"),
        ("eth", "ETH"),
        ("ethereum", "ETH"),
        ("sol", "SOL"),
        ("solana", "SOL"),
        ("etf", "ETF"),
        ("sec", "SEC"),
        ("defi", "DEFI"),
        ("hack", "RISK"),
        ("exploit", "RISK"),
        ("liquidation", "RISK"),
    ]);

    let lowered = title.to_lowercase();
    let mut tags = Vec::new();

    for (needle, tag) in rules {
        if lowered.contains(needle) && !tags.iter().any(|existing| existing == tag) {
            tags.push(tag.to_string());
        }
    }

    if tags.is_empty() {
        tags.push("MACRO".to_string());
    }

    tags
}

fn detect_severity(title: &str) -> Severity {
    let lowered = title.to_lowercase();

    if [
        "hack",
        "exploit",
        "lawsuit",
        "liquidation",
        "breach",
        "attack",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
    {
        Severity::High
    } else if ["etf", "sec", "approval", "listing", "funding", "treasury"]
        .iter()
        .any(|needle| lowered.contains(needle))
    {
        Severity::Medium
    } else {
        Severity::Info
    }
}
