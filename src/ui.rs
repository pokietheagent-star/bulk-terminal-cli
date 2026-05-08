use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Cell, List, ListItem, Paragraph, Row, Sparkline, Table, Wrap,
};

use crate::app::{App, SortMode};
use crate::market::{AlertSeverity, MarketRow};
use crate::news::Severity;

pub fn draw(frame: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(20),
            Constraint::Length(11),
            Constraint::Length(2),
        ])
        .split(frame.area());

    draw_header(frame, app, root[0]);
    draw_main(frame, app, root[1]);
    draw_bottom(frame, app, root[2]);
    draw_footer(frame, app, root[3]);
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = match app.selected_market() {
        Some(market) => format!(
            " BULK / TERMINAL  {}  tf {}  last {:.2}  mark {:.2}  funding {:+.2}bps  oi {:.1}M ",
            market.symbol,
            app.timeframe.label(),
            market.last_price,
            market.mark_price,
            market.funding_bps,
            market.open_interest_m
        ),
        None => " BULK / TERMINAL ".to_string(),
    };

    frame.render_widget(
        Paragraph::new(title)
            .block(Block::default().borders(Borders::ALL).title("Monitor"))
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        area,
    );
}

fn draw_main(frame: &mut Frame, app: &App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(24),
            Constraint::Percentage(46),
            Constraint::Percentage(30),
        ])
        .split(area);

    draw_left(frame, app, columns[0]);
    draw_center(frame, app, columns[1]);
    draw_right(frame, app, columns[2]);
}

fn draw_left(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(8)])
        .split(area);

    draw_watchlist(frame, app, rows[0]);
    draw_left_status(frame, app, rows[1]);
}

fn draw_watchlist(frame: &mut Frame, app: &App, area: Rect) {
    let rows = app.markets.iter().enumerate().map(|(idx, market)| {
        let style = if idx == app.selected_symbol {
            Style::default().fg(Color::Black).bg(Color::Cyan)
        } else {
            Style::default()
        };

        let change_color = if market.change_pct >= 0.0 {
            Color::Green
        } else {
            Color::Red
        };
        let spread_bps = if market.last_price > 0.0 {
            ((market.best_ask - market.best_bid) / market.last_price) * 10_000.0
        } else {
            0.0
        };

        Row::new(vec![
            Cell::from(market.base_symbol.clone()),
            Cell::from(format!("{:.0}", market.last_price)),
            Cell::from(format!("{:+.1}", market.change_pct))
                .style(Style::default().fg(change_color)),
            Cell::from(format!("{:.1}", spread_bps)),
            Cell::from(format!("{:+.1}", market.funding_bps)),
        ])
        .style(style)
    });

    let sort_label = match app.sort_mode {
        SortMode::Symbol => "sym",
        SortMode::Move => "move",
        SortMode::Funding => "fund",
        SortMode::OpenInterest => "oi",
        SortMode::Spread => "spr",
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Length(5),
            Constraint::Length(6),
        ],
    )
    .header(
        Row::new(vec!["Mkt", "Last", "24h", "Spr", "Fnd"]).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!("Markets [{}]", sort_label)),
    );

    frame.render_widget(table, area);
}

fn draw_left_status(frame: &mut Frame, app: &App, area: Rect) {
    let text = match app.selected_market() {
        Some(market) => vec![
            Line::from(vec![
                Span::styled(
                    format!("{:<6}", market.base_symbol),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {:.2}", market.last_price),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(format!(
                "Spread   {:>8.2}   Vol {:>7.1}M",
                market.best_ask - market.best_bid,
                market.volume_24h_m
            )),
            Line::from(format!(
                "OI       {:>8.1}M  Gap {:>+6.2}bps",
                market.open_interest_m, market.oracle_gap_bps
            )),
            Line::from(format!(
                "Depth    {:>6.0}k / {:>6.0}k",
                market.bid_depth_k, market.ask_depth_k
            )),
            Line::from(format!(
                "Bias     {}",
                depth_bias_label(market.bid_depth_k, market.ask_depth_k)
            )),
        ],
        None => vec![Line::from("Waiting for market...")],
    };

    frame.render_widget(
        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Selected"))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_center(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(9),
        ])
        .split(area);

    draw_center_summary(frame, app, rows[0]);
    draw_candle_chart(frame, app, rows[1]);
    draw_center_studies(frame, app, rows[2]);
}

fn draw_center_summary(frame: &mut Frame, app: &App, area: Rect) {
    let text = match app.selected_market() {
        Some(market) => vec![
            Line::from(vec![
                stat_span("Last", format!("{:.2}", market.last_price), Color::White),
                Span::raw("   "),
                stat_span("Mark", format!("{:.2}", market.mark_price), Color::White),
                Span::raw("   "),
                stat_span("Bid", format!("{:.2}", market.best_bid), Color::Green),
                Span::raw("   "),
                stat_span("Ask", format!("{:.2}", market.best_ask), Color::Red),
            ]),
            Line::from(vec![
                stat_span(
                    "Funding",
                    format!("{:+.2}bps", market.funding_bps),
                    Color::Yellow,
                ),
                Span::raw("   "),
                stat_span(
                    "OI",
                    format!("{:.1}M", market.open_interest_m),
                    Color::White,
                ),
                Span::raw("   "),
                stat_span("Vol", format!("{:.1}M", market.volume_24h_m), Color::White),
            ]),
            Line::from(vec![
                stat_span(
                    "Bias",
                    depth_bias_label(market.bid_depth_k, market.ask_depth_k),
                    Color::Cyan,
                ),
                Span::raw("   "),
                stat_span(
                    "Gap",
                    format!("{:+.2}bps", market.oracle_gap_bps),
                    Color::White,
                ),
            ]),
        ],
        None => vec![Line::from("Waiting for BULK feed...")],
    };

    frame.render_widget(
        Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Overview"))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn draw_candle_chart(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!("Chart [{}]", app.timeframe.label()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(market) = app.selected_market() {
        let text = render_candle_text(
            market,
            app.timeframe.sample_stride(),
            inner.width as usize,
            inner.height as usize,
        );
        frame.render_widget(Paragraph::new(text), inner);
    } else {
        frame.render_widget(Paragraph::new("Waiting for price history..."), inner);
    }
}

fn draw_center_studies(frame: &mut Frame, app: &App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(area);

    if let Some(market) = app.selected_market() {
        draw_study_sparkline(
            frame,
            columns[0],
            "Funding",
            &market.funding_history,
            market.funding_bps,
            Color::Yellow,
            |value| format!("{:+.2}bps", value),
        );
        draw_study_sparkline(
            frame,
            columns[1],
            "Open Interest",
            &market.oi_history,
            market.open_interest_m,
            Color::Cyan,
            |value| format!("{:.1}M", value),
        );
        draw_study_sparkline(
            frame,
            columns[2],
            "Oracle Gap",
            &market.gap_history,
            market.oracle_gap_bps,
            Color::LightMagenta,
            |value| format!("{:+.2}bps", value),
        );
    } else {
        for (idx, title) in ["Funding", "Open Interest", "Oracle Gap"]
            .iter()
            .enumerate()
        {
            frame.render_widget(
                Paragraph::new("Waiting for study data...")
                    .block(Block::default().borders(Borders::ALL).title(*title)),
                columns[idx],
            );
        }
    }
}

fn draw_right(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(16), Constraint::Min(10)])
        .split(area);

    draw_ladder(frame, app, rows[0]);
    draw_tape(frame, app, rows[1]);
}

fn draw_ladder(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(market) = app.selected_market() {
        let row_count = area.height.saturating_sub(4) as usize;
        let ask_levels = market
            .book
            .asks
            .iter()
            .take(row_count / 2)
            .collect::<Vec<_>>();
        let bid_levels = market
            .book
            .bids
            .iter()
            .take(row_count / 2)
            .collect::<Vec<_>>();

        let ask_max = ask_levels
            .iter()
            .map(|level| level.size)
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let bid_max = bid_levels
            .iter()
            .map(|level| level.size)
            .fold(0.0_f64, f64::max)
            .max(1.0);

        let mut rows = Vec::new();
        let mut ask_depth = 0.0;
        let depth_bar_width = 12;
        for idx in (0..ask_levels.len()).rev() {
            let level = ask_levels[idx];
            ask_depth += level.size;
            let intensity = level.size / ask_max;
            rows.push(
                Row::new(vec![
                    Cell::from(""),
                    Cell::from(format!("{:>10.2}", level.price))
                        .style(Style::default().fg(Color::Red)),
                    depth_bar_cell(level.size, intensity, depth_bar_width, false, true),
                    depth_bar_cell(ask_depth, intensity * 0.85, depth_bar_width, false, false),
                ])
                .height(1),
            );
        }

        rows.push(
            Row::new(vec![
                Cell::from(format!("{:>8.3}", market.bid_depth_k / 1_000.0)).style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(format!("{:>10.2}", market.last_price)).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from(format!("{:>8.3}", market.ask_depth_k / 1_000.0))
                    .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Cell::from(format!(
                    "{:>9.2}",
                    (market.bid_depth_k + market.ask_depth_k) / 2.0
                ))
                .style(Style::default().fg(Color::DarkGray)),
            ])
            .style(Style::default().bg(Color::Rgb(24, 24, 24))),
        );

        let mut bid_depth = 0.0;
        for level in bid_levels {
            bid_depth += level.size;
            let intensity = level.size / bid_max;
            rows.push(
                Row::new(vec![
                    depth_bar_cell(level.size, intensity, depth_bar_width, true, true),
                    Cell::from(format!("{:>10.2}", level.price))
                        .style(Style::default().fg(Color::Green)),
                    Cell::from(""),
                    depth_bar_cell(bid_depth, intensity * 0.85, depth_bar_width, true, false),
                ])
                .height(1),
            );
        }

        let table = Table::new(
            rows,
            [
                Constraint::Length(12),
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Length(12),
            ],
        )
        .header(
            Row::new(vec!["Size", "Price", "Size", "Depth"]).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(Block::default().borders(Borders::ALL).title("Ladder"))
        .column_spacing(1);

        frame.render_widget(table, area);
    } else {
        frame.render_widget(
            List::new(vec![ListItem::new("Waiting for order book...")])
                .block(Block::default().borders(Borders::ALL).title("Ladder")),
            area,
        );
    }
}

fn heat_color(is_bid: bool, intensity: f64) -> Color {
    let clamped = intensity.clamp(0.0, 1.0);
    if is_bid {
        let base = 22.0;
        let boost = 185.0 * clamped;
        Color::Rgb(
            10,
            (base + boost * 0.95).round() as u8,
            (8.0 + boost * 0.22).round() as u8,
        )
    } else {
        let base = 22.0;
        let boost = 175.0 * clamped;
        Color::Rgb(
            (base + boost).round() as u8,
            (12.0 + boost * 0.20).round() as u8,
            12,
        )
    }
}

fn depth_bar_cell(
    value: f64,
    intensity: f64,
    width: usize,
    is_bid: bool,
    bold: bool,
) -> Cell<'static> {
    let filled = ((width as f64) * intensity.clamp(0.0, 1.0)).round() as usize;
    let filled = filled.min(width);
    let bar = "█".repeat(filled);
    let padding = " ".repeat(width.saturating_sub(filled));

    let fg = if is_bid {
        Color::Rgb(220, 255, 220)
    } else {
        Color::Rgb(255, 225, 225)
    };
    let bar_color = heat_color(is_bid, intensity.max(0.22));

    let style = if bold {
        Style::default().fg(bar_color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(bar_color)
    };

    Cell::from(Line::from(vec![
        Span::styled(bar, style),
        Span::raw(padding),
        Span::raw(" "),
        Span::styled(
            format!("{value:>7.2}"),
            Style::default().fg(fg).add_modifier(if bold {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ),
    ]))
}

fn draw_tape(frame: &mut Frame, app: &App, area: Rect) {
    let items = app
        .selected_tape()
        .into_iter()
        .take(area.height.saturating_sub(2) as usize)
        .map(|trade| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<8}", trade.time_label),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{:<4}", trade.side_label),
                    Style::default().fg(if trade.is_buy {
                        Color::Green
                    } else {
                        Color::Red
                    }),
                ),
                Span::raw(format!(" {:>9.2}", trade.price)),
                Span::raw(format!(" {:>7.3}", trade.size)),
            ]))
        })
        .collect::<Vec<_>>();

    frame.render_widget(
        List::new(if items.is_empty() {
            vec![ListItem::new("Waiting for trades...")]
        } else {
            items
        })
        .block(Block::default().borders(Borders::ALL).title("Tape")),
        area,
    );
}

fn draw_bottom(frame: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    draw_alerts(frame, app, cols[0]);
    draw_news(frame, app, cols[1]);
}

fn draw_alerts(frame: &mut Frame, app: &App, area: Rect) {
    let items = app
        .selected_alerts()
        .into_iter()
        .take(area.height.saturating_sub(2) as usize)
        .map(|alert| {
            let color = match alert.severity {
                AlertSeverity::Info => Color::White,
                AlertSeverity::Warning => Color::Yellow,
                AlertSeverity::Critical => Color::Red,
            };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("{} ", alert.time_label),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        format!("{} {}", alert.symbol, alert.label),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(Span::styled(
                    alert.detail.clone(),
                    Style::default().fg(color),
                )),
            ])
        })
        .collect::<Vec<_>>();

    frame.render_widget(
        List::new(if items.is_empty() {
            vec![ListItem::new("No active alerts")]
        } else {
            items
        })
        .block(Block::default().borders(Borders::ALL).title("Alerts")),
        area,
    );
}

fn draw_news(frame: &mut Frame, app: &App, area: Rect) {
    let filtered = app.filtered_news();
    let items = filtered
        .iter()
        .enumerate()
        .take(area.height.saturating_sub(2) as usize)
        .map(|(idx, item)| {
            let color = match item.severity {
                Severity::Info => Color::White,
                Severity::Medium => Color::Yellow,
                Severity::High => Color::Red,
            };
            let prefix = if idx == app.selected_news { ">" } else { " " };
            let tag_line = if item.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", item.tags.join(","))
            };
            let link_line = item
                .link
                .trim_start_matches("https://")
                .trim_start_matches("http://");

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(prefix, Style::default().fg(Color::Cyan)),
                    Span::styled(
                        format!(" {} | {}", item.source, item.published_at),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
                Line::from(Span::styled(
                    format!("{}{}", item.title, tag_line),
                    Style::default().fg(color),
                )),
                Line::from(Span::styled(
                    link_line.to_string(),
                    Style::default().fg(Color::DarkGray),
                )),
            ])
        })
        .collect::<Vec<_>>();

    let title = format!(
        "News [{}|{}]",
        if app.active_symbol_only { "sym" } else { "all" },
        if app.severe_only { "sev" } else { "full" }
    );

    frame.render_widget(
        List::new(if items.is_empty() {
            vec![ListItem::new("Waiting for RSS headlines...")]
        } else {
            items
        })
        .block(Block::default().borders(Borders::ALL).title(title)),
        area,
    );
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let market_time = app
        .last_market_update
        .map(|time| time.format("%H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "-".to_string());

    let news_time = app
        .last_news_update
        .map(|time| time.format("%H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "-".to_string());

    let line = Line::from(vec![
        Span::raw("q quit  "),
        Span::raw("j/k markets  "),
        Span::raw("g/m/f/o/p sort  "),
        Span::raw("1 5 t y timeframe  "),
        Span::raw("a symbol filter  "),
        Span::raw("s severity filter  "),
        Span::styled(
            format!("market:{} ({})  ", app.market_status, market_time),
            Style::default().fg(Color::Green),
        ),
        Span::styled(
            format!("news:{} ({})", app.news_status, news_time),
            Style::default().fg(Color::Yellow),
        ),
    ]);

    frame.render_widget(
        Paragraph::new(line).block(Block::default().borders(Borders::ALL)),
        area,
    );
}

fn stat_span(label: &str, value: String, color: Color) -> Span<'static> {
    Span::styled(
        format!("{} {}", label, value),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

fn depth_bias_label(bid_depth_k: f64, ask_depth_k: f64) -> String {
    let ratio = bid_depth_k / ask_depth_k.max(1.0);
    if ratio > 1.15 {
        format!("bid-heavy {:.2}", ratio)
    } else if ratio < 0.87 {
        format!("ask-heavy {:.2}", ratio)
    } else {
        format!("balanced {:.2}", ratio)
    }
}

fn render_candle_text(
    market: &MarketRow,
    sample_stride: usize,
    width: usize,
    height: usize,
) -> Text<'static> {
    let inner_width = width.saturating_sub(8).max(10);
    let candle_count = (inner_width / 2).clamp(8, 24);
    let chart_height = height.saturating_sub(1).max(6);
    let candles = build_micro_candles(&market.price_history, candle_count, sample_stride);

    if candles.is_empty() {
        return Text::from("Waiting for chart data...");
    }

    let min_price = candles
        .iter()
        .map(|c| c.low)
        .fold(f64::INFINITY, |acc, value| acc.min(value));
    let max_price = candles
        .iter()
        .map(|c| c.high)
        .fold(f64::NEG_INFINITY, |acc, value| acc.max(value));
    let range = (max_price - min_price).max(1e-9);

    let mut grid = vec![vec![CellGlyph::blank(); candle_count]; chart_height];
    for (idx, candle) in candles.iter().enumerate() {
        let open_row = price_to_row(candle.open, min_price, range, chart_height);
        let close_row = price_to_row(candle.close, min_price, range, chart_height);
        let high_row = price_to_row(candle.high, min_price, range, chart_height);
        let low_row = price_to_row(candle.low, min_price, range, chart_height);

        let top_body = open_row.min(close_row);
        let bottom_body = open_row.max(close_row);
        let bullish = candle.close >= candle.open;
        let body_ch = if bullish { '#' } else { '=' };
        let body_color = if bullish { Color::Green } else { Color::Red };

        for row in high_row.min(low_row)..=high_row.max(low_row) {
            grid[row][idx] = CellGlyph {
                ch: '|',
                color: Color::DarkGray,
            };
        }
        for row in top_body..=bottom_body {
            grid[row][idx] = CellGlyph {
                ch: body_ch,
                color: body_color,
            };
        }
    }

    let mut lines = Vec::new();
    for (row_idx, row) in grid.iter().enumerate() {
        let price_label =
            max_price - (range * row_idx as f64 / (chart_height.saturating_sub(1).max(1) as f64));
        let mut spans = vec![Span::styled(
            format!("{:>7.2} ", price_label),
            Style::default().fg(Color::DarkGray),
        )];
        for cell in row {
            spans.push(Span::styled(
                format!("{} ", cell.ch),
                Style::default().fg(cell.color),
            ));
        }
        lines.push(Line::from(spans));
    }
    Text::from(lines)
}

#[derive(Clone, Copy)]
struct CellGlyph {
    ch: char,
    color: Color,
}

impl CellGlyph {
    fn blank() -> Self {
        Self {
            ch: ' ',
            color: Color::Reset,
        }
    }
}

struct MicroCandle {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
}

fn build_micro_candles(
    history: &[f64],
    target_candles: usize,
    sample_stride: usize,
) -> Vec<MicroCandle> {
    if history.is_empty() || target_candles == 0 {
        return Vec::new();
    }

    let effective = history
        .iter()
        .step_by(sample_stride.max(1))
        .copied()
        .collect::<Vec<_>>();
    let bucket_size = ((effective.len() as f64) / (target_candles as f64)).ceil() as usize;
    effective
        .chunks(bucket_size.max(1))
        .take(target_candles)
        .filter_map(|chunk| {
            let open = *chunk.first()?;
            let close = *chunk.last()?;
            let mut high = open;
            let mut low = open;
            for value in chunk {
                high = high.max(*value);
                low = low.min(*value);
            }
            Some(MicroCandle {
                open,
                high,
                low,
                close,
            })
        })
        .collect()
}

fn draw_study_sparkline<F>(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    history: &[f64],
    latest: f64,
    color: Color,
    labeler: F,
) where
    F: Fn(f64) -> String,
{
    let data = normalize_study(history);
    frame.render_widget(
        Sparkline::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(Line::from(vec![
                        Span::styled(
                            title.to_string(),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(labeler(latest), Style::default().fg(Color::White)),
                    ])),
            )
            .data(&data)
            .style(Style::default().fg(color))
            .bar_set(symbols::bar::NINE_LEVELS),
        area,
    );
}

fn normalize_study(history: &[f64]) -> Vec<u64> {
    if history.is_empty() {
        return Vec::new();
    }

    let min = history
        .iter()
        .fold(f64::INFINITY, |acc, value| acc.min(*value));
    let max = history
        .iter()
        .fold(f64::NEG_INFINITY, |acc, value| acc.max(*value));
    let range = (max - min).max(1e-9);

    history
        .iter()
        .map(|value| (((value - min) / range) * 100.0).round() as u64)
        .collect()
}

fn price_to_row(price: f64, min_price: f64, range: f64, chart_height: usize) -> usize {
    let normalized = ((price - min_price) / range).clamp(0.0, 1.0);
    let inverted = 1.0 - normalized;
    ((inverted * (chart_height.saturating_sub(1) as f64)).round() as usize)
        .min(chart_height.saturating_sub(1))
}
