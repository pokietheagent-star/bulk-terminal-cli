# Bulk MM CLI

Command-line market maker for [Bulk](https://docs.bulk.trade/api-reference/introduction). One market (`BTC-USD` by default). **Dry-run is the default and places nothing.**

**Not financial advice. Market making can lose money. Dry-run first, then testnet.**

## Clone and dry-run

```bash
npm install && npm run dry-run
```

Five ticks against testnet public data. Prints mid + would-be ALO bid/ask. No keys. No `POST /order`.

The CLI lives in [`mm-cli/`](mm-cli/). Full flags, network switch, faucet, and the mainnet gate are documented there.

```bash
npx --prefix mm-cli tsx mm-cli/src/cli.ts run --network testnet
npx --prefix mm-cli tsx mm-cli/src/cli.ts network set testnet
npx --prefix mm-cli tsx mm-cli/src/cli.ts faucet --network testnet
```

Live mainnet requires **both** `--enable-mainnet` and `BULK_ALLOW_MAINNET=1`.

Keys are env-only. See [`.env.example`](.env.example). Never commit secrets.

## Also in this repo

The existing Rust watchlist is unchanged. See [MONITOR.md](MONITOR.md) (`cargo run`). It is not this market-making CLI.
