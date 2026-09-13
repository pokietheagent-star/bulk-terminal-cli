import { printSafetyBanner } from "./banner.js";
import { BulkClient, NETWORKS } from "./bulk/client.js";
import {
  aloLimit,
  cancelAll,
  loadTradingKeys,
  signCancelAll,
  signQuoteRefresh,
  type TradingKeys,
} from "./bulk/signing.js";
import type { AccountSnapshot } from "./bulk/types.js";
import type { RunConfig } from "./config.js";
import { formatPx, redactSecrets } from "./log.js";
import { computeQuotes, marketQuoteInputs, midFromBook, midFromTicker, midMovedBps } from "./quotes.js";
import { assertMainnetLiveAllowed, shouldHardKill } from "./safety.js";

export interface EngineIo {
  log?: (line: string) => void;
  error?: (line: string) => void;
  now?: () => number;
  sleep?: (ms: number) => Promise<void>;
  client?: BulkClient;
}

export async function runEngine(config: RunConfig, io: EngineIo = {}): Promise<void> {
  const log = io.log ?? ((line: string) => console.log(line));
  const error = io.error ?? ((line: string) => console.error(line));
  const sleep = io.sleep ?? ((ms: number) => new Promise((resolve) => setTimeout(resolve, ms)));

  assertMainnetLiveAllowed({
    network: config.network,
    mode: config.mode,
    enableMainnet: config.enableMainnet,
    allowMainnetEnv: process.env.BULK_ALLOW_MAINNET,
  });

  const endpoints = NETWORKS[config.network];
  const client = io.client ?? new BulkClient(endpoints.http);
  const live = config.mode === "live";
  const keys = live ? loadTradingKeys(config.network) : undefined;
  const account = keys?.account ?? process.env.BULK_ACCOUNT?.trim();

  printSafetyBanner(log);
  log(`bulk-mm  market=${config.market}  network=${config.network}  mode=${config.mode}`);
  log(`http=${endpoints.http}`);
  log(
    `size=${config.size}  spreadBps=${config.spreadBps}  requoteThresholdBps=${config.requoteThresholdBps}`
      + (config.requoteThresholdDefaulted ? " (default = half of spreadBps, min 1)" : ""),
  );
  log(
    `inventorySkewStrength=${config.inventorySkewStrength}  maxPosition=${config.maxPosition}  maxLossUsd=${config.maxLossUsd}`,
  );
  if (!live) {
    log("dry-run: planned place/cancel only. Never POST /order.");
  }

  const market = await client.market(config.market);
  if (market.status && market.status !== "TRADING") {
    throw new Error(`market ${config.market} status is ${market.status}`);
  }

  let lastQuotedMid: number | undefined;
  let resting = false;
  let ticks = 0;
  let stopping = false;

  const shutdown = async (reason: string): Promise<void> => {
    if (stopping) {
      return;
    }
    stopping = true;
    log(`hard kill: ${reason}`);
    await cancelOutstanding({ live, keys, client, config, log, resting });
    resting = false;
  };

  const onSignal = (signal: string) => {
    void shutdown(`${signal}`).finally(() => process.exit(0));
  };
  process.once("SIGINT", () => onSignal("SIGINT"));
  process.once("SIGTERM", () => onSignal("SIGTERM"));

  try {
    while (!stopping) {
      ticks += 1;
      const snapshot = await readInventory(client, config, account);
      const kill = shouldHardKill({
        position: snapshot.position,
        maxPosition: config.maxPosition,
        totalPnl: snapshot.totalPnl,
        maxLossUsd: config.maxLossUsd,
      });
      if (kill.kill && kill.reason) {
        await shutdown(kill.reason);
        break;
      }

      const mid = await readMid(client, config.market);
      const moveBps = lastQuotedMid === undefined ? Number.POSITIVE_INFINITY : midMovedBps(lastQuotedMid, mid);
      const shouldRequote = lastQuotedMid === undefined || moveBps >= config.requoteThresholdBps;

      const quotes = computeQuotes({
        mid,
        size: config.size,
        spreadBps: config.spreadBps,
        inventorySkewStrength: config.inventorySkewStrength,
        position: snapshot.position,
        maxPosition: config.maxPosition,
        ...marketQuoteInputs(market),
      });

      if (!shouldRequote) {
        log(
          `${prefix(config)} tick=${ticks} mid=${formatPx(mid)} hold `
            + `(move ${moveBps.toFixed(2)} bps < requote ${config.requoteThresholdBps} bps) `
            + `pos=${snapshot.position} pnlUsd=${snapshot.totalPnl.toFixed(2)}`,
        );
      } else {
        log(
          `${prefix(config)} tick=${ticks} mid=${formatPx(mid)} `
            + `ALO bid=${formatPx(quotes.bidPx)} ask=${formatPx(quotes.askPx)} sz=${quotes.size} `
            + `invRatio=${quotes.inventoryRatio.toFixed(3)} pos=${snapshot.position} pnlUsd=${snapshot.totalPnl.toFixed(2)}`,
        );
        await placeQuotes({
          live,
          keys,
          client,
          config,
          quotes,
          log,
          resting,
        });
        lastQuotedMid = mid;
        resting = true;
      }

      if (config.ticks !== undefined && ticks >= config.ticks) {
        await shutdown("tick limit");
        break;
      }
      await sleep(config.intervalMs);
    }
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    error(redactSecrets(message));
    await cancelOutstanding({ live, keys, client, config, log, resting }).catch((cancelErr) => {
      error(redactSecrets(cancelErr instanceof Error ? cancelErr.message : String(cancelErr)));
    });
    throw err;
  }
}

async function readMid(client: BulkClient, symbol: string): Promise<number> {
  try {
    const book = await client.l2book(symbol);
    const mid = midFromBook(book);
    if (mid !== undefined) {
      return mid;
    }
  } catch {
    // Fall through to ticker.
  }
  const ticker = await client.ticker(symbol);
  const mid = midFromTicker(ticker);
  if (mid === undefined) {
    throw new Error(`no mid available for ${symbol}`);
  }
  return mid;
}

async function readInventory(
  client: BulkClient,
  config: RunConfig,
  account?: string,
): Promise<AccountSnapshot> {
  if (config.simulatePosition !== undefined) {
    return {
      position: config.simulatePosition,
      realizedPnl: 0,
      unrealizedPnl: 0,
      totalPnl: 0,
    };
  }
  if (!account) {
    return { position: 0, realizedPnl: 0, unrealizedPnl: 0, totalPnl: 0 };
  }
  try {
    return await client.account(account, config.market);
  } catch {
    return { position: 0, realizedPnl: 0, unrealizedPnl: 0, totalPnl: 0 };
  }
}

async function placeQuotes(input: {
  live: boolean;
  keys?: TradingKeys;
  client: BulkClient;
  config: RunConfig;
  quotes: ReturnType<typeof computeQuotes>;
  log: (line: string) => void;
  resting: boolean;
}): Promise<void> {
  const actions = [
    cancelAll(input.config.market),
    aloLimit({
      symbol: input.config.market,
      isBuy: true,
      price: input.quotes.bidPx,
      size: input.quotes.size,
    }),
    aloLimit({
      symbol: input.config.market,
      isBuy: false,
      price: input.quotes.askPx,
      size: input.quotes.size,
    }),
  ];

  if (!input.live) {
    if (input.resting) {
      input.log(`  would cancel previous quotes then re-quote both sides (cxa + ALO bid/ask)`);
    } else {
      input.log(`  would POST /order batch: cxa ${input.config.market} + ALO bid + ALO ask`);
    }
    input.log("  no orders posted");
    return;
  }

  if (!input.keys) {
    throw new Error("live mode requires trading keys");
  }
  const envelope = signQuoteRefresh(input.keys, input.config.network, actions);
  const response = await input.client.submit(envelope);
  input.log(`  posted batch cancel+place nonce=${envelope.nonce}`);
  input.log(`  exchange=${summarizeResponse(response)}`);
}

async function cancelOutstanding(input: {
  live: boolean;
  keys?: TradingKeys;
  client: BulkClient;
  config: RunConfig;
  log: (line: string) => void;
  resting: boolean;
}): Promise<void> {
  if (!input.live) {
    input.log(`  would cancel-all (cxa ${input.config.market}) then exit`);
    return;
  }
  if (!input.keys) {
    return;
  }
  const envelope = signCancelAll(input.keys, input.config.network, input.config.market);
  const response = await input.client.submit(envelope);
  input.log(`  posted cxa ${input.config.market} nonce=${envelope.nonce}`);
  input.log(`  exchange=${summarizeResponse(response)}`);
}

function prefix(config: RunConfig): string {
  return config.mode === "dry-run" ? "[dry-run]" : "[live]";
}

function summarizeResponse(response: unknown): string {
  try {
    return JSON.stringify(response);
  } catch {
    return String(response);
  }
}
