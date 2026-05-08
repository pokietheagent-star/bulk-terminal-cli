# BULK Monitor

A CLI-first market monitor built for BULK.

Current MVP:

- terminal watchlist for `BTC-USD`, `ETH-USD`, and `SOL-USD`
- live BULK websocket market feed for ticker and L2 snapshots
- real order book pane
- trade tape pane
- local alert stream for funding, spread, imbalance, oracle gap, and OI moves
- RSS-backed crypto news pane
- symbol-aware headline filtering
- severity highlighting for risk and policy headlines
- automatic fallback to simulated market data if BULK is unreachable

## Controls

- `q`: quit
- `j` / `k`: move through markets
- `h` / `l`: move through headlines
- `a`: toggle active-symbol-only news filter
- `s`: toggle medium/high severity filter
- `g`: sort watchlist by symbol
- `m`: sort watchlist by move
- `f`: sort watchlist by funding
- `o`: sort watchlist by open interest
- `p`: sort watchlist by spread
- `1`: 1m chart view
- `5`: 5m chart view
- `t`: 15m chart view
- `y`: 60m chart view

## Run

```bash
cargo run
```

## Free RSS Feeds

- CoinDesk: `https://www.coindesk.com/arc/outboundfeeds/rss/`
- Cointelegraph: `https://cointelegraph.com/rss.xml`

## Notes

The market worker connects to `wss://exchange-wss.bulk.trade` through the Rust `bulk-client`
SDK, subscribes to ticker updates for the watchlist, and subscribes to L2 snapshots for
order-book depth.
