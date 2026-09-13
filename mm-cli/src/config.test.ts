import assert from "node:assert/strict";
import { afterEach, describe, it } from "node:test";
import { resolveRunConfig } from "./config.js";

const ENV_KEYS = [
  "BULK_MARKET",
  "BULK_SIZE",
  "BULK_SPREAD_BPS",
  "BULK_REQUOTE_THRESHOLD_BPS",
  "BULK_INVENTORY_SKEW_STRENGTH",
  "BULK_MAX_POSITION",
  "BULK_MAX_LOSS_USD",
  "BULK_MODE",
  "BULK_NETWORK",
  "BULK_INTERVAL_MS",
] as const;

const saved = Object.fromEntries(ENV_KEYS.map((key) => [key, process.env[key]]));

afterEach(() => {
  for (const key of ENV_KEYS) {
    if (saved[key] === undefined) {
      delete process.env[key];
    } else {
      process.env[key] = saved[key];
    }
  }
});

describe("resolveRunConfig", () => {
  for (const key of ENV_KEYS) {
    delete process.env[key];
  }
  it("defaults to dry-run, BTC-USD, and half-spread requote", () => {
    const config = resolveRunConfig({});
    assert.equal(config.mode, "dry-run");
    assert.equal(config.market, "BTC-USD");
    assert.equal(config.spreadBps, 10);
    assert.equal(config.requoteThresholdBps, 5);
    assert.equal(config.requoteThresholdDefaulted, true);
    assert.equal(config.network, "testnet");
  });

  it("prefers persisted network when nothing else is set", () => {
    const config = resolveRunConfig({}, "mainnet");
    assert.equal(config.network, "mainnet");
    assert.equal(config.mode, "dry-run");
  });

  it("uses an explicit requote threshold when provided", () => {
    const config = resolveRunConfig({ requoteThresholdBps: 2, spreadBps: 20 });
    assert.equal(config.requoteThresholdBps, 2);
    assert.equal(config.requoteThresholdDefaulted, false);
  });
});
