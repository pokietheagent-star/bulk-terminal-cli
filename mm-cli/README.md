# bulk-mm

Single-market market-making CLI for [Bulk](https://docs.bulk.trade/api-reference/introduction). Dry-run is the default and **places nothing**.

**Not financial advice. Market making can lose money. Dry-run first, then testnet.**

This package is the MVP. The repo also has an unrelated Rust watchlist (`cargo run` from the repo root). That keeper is unchanged.

## One-command dry-run

From the repo root:

```bash
npm install && npm run dry-run
```

Or from this directory:

```bash
npm install && npm run dry-run
```

That runs five ticks on **testnet public market data** for `BTC-USD`. It prints mid + would-be ALO bid/ask. It never calls `POST /order`. No keys required.

Continuous dry-run:

```bash
npx tsx src/cli.ts run --network testnet
# Ctrl-C cancels in name only (prints would-be cxa) and exits
```

## Network switch

```bash
npx tsx src/cli.ts run --network testnet
npx tsx src/cli.ts run --network mainnet          # dry-run is still default
npx tsx src/cli.ts network set testnet            # persist default (~/.config/bulk-mm/config.json)
npx tsx src/cli.ts network get
```

`--network` wins, then `BULK_NETWORK`, then the persisted default, then `testnet`.

Dry-run works on **both** networks (public ticker / L2 only). Live mainnet is a separate gate.

## Testnet live + faucet

1. Copy `.env.example` to `.env` and set keys (env only; never commit secrets).
2. Claim paper funds. Faucet is **signed**, **testnet-only**, about **10k / 24h**.

```bash
npx tsx src/cli.ts faucet --network testnet
```

3. Live-practice quotes (keys required; still not mainnet):

```bash
npx tsx src/cli.ts run --mode live --network testnet
```

Live cancels previous quotes and places a new ALO bid+ask in one signed batch (`cxa` + two `l` ALO actions) via `bulk-keychain`.

## Mainnet gate

Live mainnet is refused unless **both** are set:

1. `--enable-mainnet`
2. `BULK_ALLOW_MAINNET=1`

```bash
# refused (missing env and/or flag)
npx tsx src/cli.ts run --mode live --network mainnet

# only this combination can place on mainnet
BULK_ALLOW_MAINNET=1 npx tsx src/cli.ts run --mode live --network mainnet --enable-mainnet
```

Dry-run on mainnet does **not** need the gate. It still places nothing.

## Config

| Knob | Flag / env | Default |
| --- | --- | --- |
| market | `--market` / `BULK_MARKET` | `BTC-USD` |
| size | `--size` / `BULK_SIZE` | `0.001` |
| spreadBps | `--spread-bps` / `BULK_SPREAD_BPS` | `10` |
| requoteThresholdBps | `--requote-threshold-bps` / `BULK_REQUOTE_THRESHOLD_BPS` | **half of `spreadBps`, minimum 1** |
| inventorySkewStrength | `--inventory-skew-strength` / `BULK_INVENTORY_SKEW_STRENGTH` | `0.5` |
| maxPosition | `--max-position` / `BULK_MAX_POSITION` | `0.01` |
| maxLossUsd | `--max-loss-usd` / `BULK_MAX_LOSS_USD` | `100` (USD notional) |
| mode | `--mode dry-run\|live` / `BULK_MODE` | `dry-run` |
| network | `--network testnet\|mainnet` / `BULK_NETWORK` | `testnet` (or persisted) |
| interval | `--interval-ms` | `2000` |
| optional file | `--config path.json` | — |

`requoteThresholdBps` defaults to half the spread (min 1) so quotes rest through noise smaller than half the posted width, then both sides cancel/re-quote together when mid has actually moved.

Optional JSON file (`--config`) can set the same knobs. CLI flags win.

## Keys (env only)

| Name | Use |
| --- | --- |
| `BULK_ACCOUNT` | Trading account pubkey (base58). Live + faucet. Optional in dry-run to read inventory via unsigned `POST /account`. |
| `BULK_AGENT_SECRET` | Agent (or account) secret, base58. **Never logged in full.** |
| `BULK_AGENT_SECRET_PATH` | File with the secret (base58 or Solana JSON byte array). |
| `BULK_AGENT_PUBKEY` | Optional check; derived from the secret if omitted. |
| `BULK_ALLOW_MAINNET` | Must be `1` for live mainnet. |

If `BULK_ACCOUNT` differs from the agent pubkey, orders are signed with `prepare*` + `signPrepared` (agent-wallet path). The agent must already be authorized on the account.

## Loop

1. Read mid from public L2 (`GET /l2book`) or ticker (`GET /ticker/{symbol}`).
2. Compute ALO (post-only) bid + ask around mid: half-spread each side, then a linear inventory skew.
3. Dry-run: print planned `cxa` + ALO bid/ask. Never `POST /order`.
4. Live: one signed batch — cancel-all for the market, then place both ALO quotes (`bulk-keychain` `signGroup` / `prepareOrderGroup`).
5. Re-quote both sides when mid moves by `>= requoteThresholdBps`.

Inventory skew (plain rule, no ML): reservation price `= mid - (position / maxPosition) * strength * halfSpread`. Long inventory leans quotes down (easier to sell); short leans them up.

Hard kills cancel-all (`cxa` for the market) then exit: max position, max loss USD (realized + unrealized), Ctrl-C / SIGINT / SIGTERM.

## Official sources

- API intro, place/cancel, signing, faucet, exchangeInfo / ticker / L2 / account: [docs.bulk.trade](https://docs.bulk.trade/api-reference/introduction)
- Signing library: [`bulk-keychain`](https://github.com/Bulk-trade/bulk-keychain) (this CLI does not hand-roll wincode)
- HF MM notes: [hf-market-making](https://docs.bulk.trade/bulk-exchange/hf-market-making.md)

Base URLs (from docs):

- Mainnet HTTP `https://mainnet-api1.bulk.trade/api/v1` · WS `wss://mainnet-ws1.bulk.trade` · domain `1`
- Testnet HTTP `https://exchange-api.bulk.trade/api/v1` · WS `wss://exchange-ws1.bulk.trade` · domain `2`

## Out of scope

No GUI, multi-market, HFT / validator / co-lo, ML, or Discord/Forum. Does not wait for the HF SDK.
