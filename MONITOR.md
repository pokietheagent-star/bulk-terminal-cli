# BULK Monitor

Unrelated Rust watchlist that already lived in this repo. The market-making CLI is documented in [README.md](README.md).

Current MVP:

- terminal watchlist for `BTC-USD`, `ETH-USD`, and `SOL-USD`
- live BULK websocket market feed for ticker and L2 snapshots
- real order book pane
- trade tape pane
- local alert stream for funding, spread, imbalance, oracle gap, and OI moves
- RSS-backed crypto news pane

## Run

```bash
cargo run
```

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
- `1` / `5` / `t` / `y`: 1m / 5m / 15m / 60m chart view
