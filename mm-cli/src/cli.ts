#!/usr/bin/env node
import { config as loadEnv } from "dotenv";
import { Command, Option } from "commander";
import { faucetCommand } from "./commands/faucet.js";
import { networkGet, networkSet } from "./commands/network.js";
import { runCommand } from "./commands/run.js";
import { MainnetLiveBlockedError } from "./safety.js";

loadEnv({ quiet: true });
loadEnv({ path: "mm-cli/.env", quiet: true });

const program = new Command();
program
  .name("bulk-mm")
  .description("Bulk Trade market-making CLI. Dry-run is the default and places nothing.")
  .showHelpAfterError();

program
  .command("run")
  .description("Quote one market. Dry-run by default.")
  .option("--market <symbol>", "market symbol", "BTC-USD")
  .option("--size <n>", "order size", Number)
  .option("--spread-bps <n>", "bid/ask spread in basis points", Number)
  .option("--requote-threshold-bps <n>", "re-quote when mid moves this many bps (default: half of spread, min 1)", Number)
  .option("--inventory-skew-strength <n>", "linear inventory skew strength", Number)
  .option("--max-position <n>", "hard-kill absolute position", Number)
  .option("--max-loss-usd <n>", "hard-kill USD notional stop", Number)
  .addOption(new Option("--mode <mode>", "dry-run or live").choices(["dry-run", "live"]))
  .addOption(new Option("--network <network>", "testnet or mainnet").choices(["testnet", "mainnet"]))
  .option("--enable-mainnet", "required together with BULK_ALLOW_MAINNET=1 for live mainnet")
  .option("--interval-ms <n>", "loop interval", Number)
  .option("--ticks <n>", "exit after N loop ticks (useful for dry-run demos)", Number)
  .option("--simulate-position <n>", "dry-run inventory override for skew inspection", Number)
  .option("--config <path>", "optional JSON config file")
  .action(async (opts) => {
    await runCommand({
      market: opts.market,
      size: opts.size,
      spreadBps: opts.spreadBps,
      requoteThresholdBps: opts.requoteThresholdBps,
      inventorySkewStrength: opts.inventorySkewStrength,
      maxPosition: opts.maxPosition,
      maxLossUsd: opts.maxLossUsd,
      mode: opts.mode,
      network: opts.network,
      enableMainnet: Boolean(opts.enableMainnet),
      intervalMs: opts.intervalMs,
      ticks: opts.ticks,
      simulatePosition: opts.simulatePosition,
      config: opts.config,
    });
  });

const network = program.command("network").description("Show or persist the default network");
network.command("get").description("Print persisted / env / default network").action(networkGet);
network
  .command("set")
  .description("Persist default network (testnet|mainnet)")
  .argument("<network>", "testnet or mainnet")
  .action(networkSet);

program
  .command("faucet")
  .description("Signed testnet faucet (~10k / 24h). Requires env keys. Testnet only.")
  .addOption(new Option("--network <network>", "must be testnet").choices(["testnet", "mainnet"]))
  .action(async (opts) => {
    await faucetCommand(opts.network);
  });

program.parseAsync(process.argv).catch((err: unknown) => {
  const message = err instanceof Error ? err.message : String(err);
  console.error(message);
  process.exit(err instanceof MainnetLiveBlockedError ? 2 : 1);
});
