# BULK Monitor V2 Spec

## Goal

Build a CLI-native market monitoring terminal for BULK that helps a user answer three questions quickly:

1. What is moving?
2. What does the order flow look like?
3. Is there market news that explains it?

The product is read-only by default. It is a monitoring tool, not a trader UI.

## Product Positioning

The intended feel is:

- `Bookmap` influence: liquidity awareness and order flow
- `TradingView` influence: watchlist, symbol-centric navigation, related news
- `Bloomberg` influence: dense layout, integrated context, minimal wasted space

The terminal should feel stable, fast, and keyboard-first.

## Design Principles

- Prioritize scanability over decoration.
- Use color only when it conveys market meaning.
- Keep columns fixed-width and visually stable while values update.
- Avoid pane jumping and layout shifts.
- Prefer compact summaries over large verbose panels.
- Always keep watchlist, active market context, and news visible.

## Data Sources

### From BULK SDK

These are the primary market inputs:

- WebSocket ticker feed
  - `last_price`
  - `mark_price`
  - `oracle_price`
  - `price_change`
  - `price_change_percent`
  - `volume`
  - `quote_volume`
  - `open_interest`
  - `funding_rate`
- WebSocket L2 snapshot
  - top levels
  - best bid / ask
  - spread
  - depth by side
- WebSocket L2 delta
  - book update cadence
  - local liquidity pressure estimate
- Trades feed
  - aggressive tape
  - buy/sell burst detection
- Candles feed
  - charting
  - intraday structure

### From RSS

News inputs:

- CoinDesk RSS
- Cointelegraph RSS
- optional future additions:
  - The Block RSS
  - Decrypt RSS
  - Bitcoin Magazine RSS

### Not Included In V2

- Cross-venue prices
- liquidations API
- social feeds
- execution
- account write actions

## Core Screens

V2 uses one main screen with four persistent zones.

### 1. Left Pane: Watchlist

Purpose:

- symbol navigation
- market-wide ranking
- quick relative comparison

Columns:

- `symbol`
- `last`
- `24h %`
- `fund`
- `OI`
- `spread`

Optional later columns:

- `vol`
- `mark-oracle bps`
- `alert score`

Sort modes:

- symbol
- biggest 24h move
- highest funding
- biggest OI
- widest spread
- alert score

Visual rules:

- selected symbol highlighted in cyan
- positive price change green
- negative price change red
- high funding yellow or red
- stale symbols dimmed

### 2. Center Pane: Active Market Summary

Purpose:

- show the selected market’s state at a glance

Fields:

- symbol
- last
- mark
- oracle
- mark-oracle divergence
- funding in bps
- open interest
- quote volume
- spread
- bid/ask depth ratio

Derived labels:

- `bid-heavy`
- `ask-heavy`
- `balanced`
- `funding-hot`
- `spread-wide`
- `oracle-gap`

Below the summary:

- compact chart area
- default view: short pulse chart from candles or price history
- alternative future views:
  - 1m candles
  - 5m candles
  - spread history
  - mark-oracle divergence history

### 3. Right Pane: Order Flow

Purpose:

- market microstructure context

Sections:

- top asks
- spread row
- top bids
- tape / recent trades

Order book details:

- top 10 levels per side
- price
- size
- optionally cumulative depth

Trade tape:

- time
- side
- size
- price

Derived order-flow metrics:

- bid depth total
- ask depth total
- imbalance ratio
- large trade count in rolling window
- burst direction

Visual rules:

- asks warm color
- bids cool/green color
- tape prints green/red by aggressor side
- large prints bold

### 4. Bottom Pane: News And Alerts

Purpose:

- explain moves and highlight notable changes

This pane is split conceptually into:

- `Alerts` stream
- `News` stream

Alerts appear first because they are directly tied to BULK data.

News behavior:

- show active-symbol headlines first
- then broader crypto headlines
- dedupe by normalized title
- include timestamp, source, headline, tags

Alert behavior:

- generated from local rules
- mixed into the same timeline or shown in a left/right split

Recommended first layout:

- top half: alerts
- bottom half: news

## Alert Engine

Alerts are the main product differentiator because they turn raw feed data into operator value.

### V2 Alert Types

#### Price Alerts

- `fast-move-up`
  - trigger when short-window move exceeds threshold
- `fast-move-down`
  - same for downside

#### Funding Alerts

- `funding-hot`
  - trigger when funding bps exceeds configured threshold
- `funding-flip`
  - trigger when funding changes sign

#### Spread Alerts

- `spread-wide`
  - trigger when spread exceeds rolling baseline multiple

#### Order Book Alerts

- `bid-imbalance`
  - trigger when bid depth materially exceeds ask depth
- `ask-imbalance`
  - opposite case
- `liquidity-drop`
  - trigger when near-touch depth disappears rapidly

#### Open Interest Alerts

- `oi-jump`
  - trigger on rapid increase
- `oi-drop`
  - trigger on rapid decrease

#### Oracle Divergence Alerts

- `oracle-gap`
  - trigger when mark diverges from oracle above threshold

#### News Alerts

- `headline-match`
  - news mentions selected symbol
- `headline-risk`
  - risk or regulatory keyword match

### Alert Severity

Three levels:

- `info`
- `warning`
- `critical`

Severity examples:

- `info`: small funding increase
- `warning`: spread 2x normal
- `critical`: risk headline plus major price move

## News Engine

### V2 RSS Features

- polling every 30 to 60 seconds
- deduping
- source display
- active-symbol prioritization
- keyword tagging
- severity tagging

### Symbol Matching Rules

Match headlines against:

- `BTC`, `Bitcoin`
- `ETH`, `Ethereum`
- `SOL`, `Solana`

Future:

- map full BULK watchlist dynamically from selected symbols

### Headline Tag Classes

- asset tags: `BTC`, `ETH`, `SOL`
- theme tags: `ETF`, `SEC`, `DEFI`, `LAYER1`, `LISTING`
- risk tags: `HACK`, `EXPLOIT`, `LIQUIDATION`, `LAWSUIT`

### News Severity Rules

- `critical`
  - exploit
  - hack
  - lawsuit
  - liquidation
  - breach
- `warning`
  - sec
  - etf
  - approval
  - treasury
  - listing
- `info`
  - everything else

## Keybindings

### Global

- `q`: quit
- `?`: help overlay
- `tab`: rotate pane focus

### Watchlist

- `j` / `k`: move symbol selection
- `g`: sort by symbol
- `m`: sort by 24h move
- `f`: sort by funding
- `o`: sort by open interest
- `p`: sort by spread

### Center Pane

- `1`: short pulse chart
- `2`: candle chart
- `3`: spread history
- `4`: mark-oracle history

### Order Flow

- `[` and `]`: adjust displayed depth levels
- `t`: toggle trade tape
- `d`: toggle cumulative depth view

### News / Alerts

- `a`: toggle active-symbol-only news
- `s`: toggle severity filter
- `n`: toggle news pane emphasis
- `r`: toggle alerts pane emphasis

## Visual Direction

### Color Semantics

- green: positive move, bids, buy aggression
- red: negative move, asks, sell aggression
- yellow: warning, elevated funding, elevated spread
- magenta: chart pulse accent
- cyan: selection, current focus, important labels
- gray: metadata, timestamps, inactive values

### Layout Behavior

- fixed pane widths by default
- no auto-resize based on content
- truncated headlines rather than pane expansion
- values right-aligned in tables
- stable order book row count

### Typography In CLI Terms

- small, dense, consistent spacing
- numeric columns aligned
- headers bold but short
- avoid long prose in-pane

## V2 Feature Matrix

### Must-Have

- live BULK ticker watchlist
- live L2 top-of-book and depth
- compact active market summary
- recent trades tape
- RSS news pane
- alert engine
- sort modes
- keyboard navigation

### Nice-To-Have

- candle chart view
- rolling metrics history
- split alerts/news sub-pane
- stale feed indicator by symbol
- reconnect status banner

### Later

- replay
- account monitoring pane
- multi-layout presets
- export snapshots

## Implementation Mapping

### Existing App Pieces

- `src/main.rs`
  - event loop
  - terminal lifecycle
- `src/ui.rs`
  - pane rendering
- `src/news.rs`
  - RSS polling
- `src/market.rs`
  - BULK worker
- `src/app.rs`
  - app state

### Needed V2 Modules

- `src/alerts.rs`
  - alert rules and scoring
- `src/orderflow.rs`
  - order book shaping, depth totals, tape metrics
- `src/history.rs`
  - rolling timeseries buffers for chart modes
- `src/input.rs`
  - keybinding actions and focus handling
- `src/layout.rs`
  - pane mode and layout presets

## Data Model Additions

### App State

Add:

- focused pane
- watchlist sort mode
- chart mode
- trade tape per symbol
- order book levels per symbol
- alert stream
- rolling history buffers

### Market State

Add:

- recent ticker history
- recent spread history
- recent mark-oracle divergence history
- L2 snapshot cache
- L2 delta application
- trade tape buffer

## Delivery Plan

### Phase 1

- refine watchlist columns
- add real order book pane
- add trade tape
- add alert engine

### Phase 2

- add chart modes
- add rolling histories
- add richer news ranking

### Phase 3

- add layout presets
- add account read-only pane
- add stronger reconnect and feed health states

## V2 Success Criteria

V2 is successful if a user can:

- identify the most interesting BULK market in under 5 seconds
- understand whether the market is orderly or stressed
- see whether current headlines are relevant to the selected symbol
- navigate entirely by keyboard
- leave the terminal open as a persistent monitor
