import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { assertMainnetLiveAllowed, MainnetLiveBlockedError, shouldHardKill } from "./safety.js";

describe("assertMainnetLiveAllowed", () => {
  it("allows dry-run on mainnet without the gate", () => {
    assert.doesNotThrow(() =>
      assertMainnetLiveAllowed({
        network: "mainnet",
        mode: "dry-run",
        enableMainnet: false,
        allowMainnetEnv: undefined,
      }),
    );
  });

  it("allows live testnet without the gate", () => {
    assert.doesNotThrow(() =>
      assertMainnetLiveAllowed({
        network: "testnet",
        mode: "live",
        enableMainnet: false,
        allowMainnetEnv: undefined,
      }),
    );
  });

  it("refuses live mainnet unless flag and env are both set", () => {
    assert.throws(
      () =>
        assertMainnetLiveAllowed({
          network: "mainnet",
          mode: "live",
          enableMainnet: true,
          allowMainnetEnv: undefined,
        }),
      MainnetLiveBlockedError,
    );
    assert.throws(
      () =>
        assertMainnetLiveAllowed({
          network: "mainnet",
          mode: "live",
          enableMainnet: false,
          allowMainnetEnv: "1",
        }),
      MainnetLiveBlockedError,
    );
    assert.doesNotThrow(() =>
      assertMainnetLiveAllowed({
        network: "mainnet",
        mode: "live",
        enableMainnet: true,
        allowMainnetEnv: "1",
      }),
    );
  });
});

describe("shouldHardKill", () => {
  it("trips on max position and max loss USD", () => {
    assert.equal(
      shouldHardKill({ position: 0.01, maxPosition: 0.01, totalPnl: 0, maxLossUsd: 100 }).kill,
      true,
    );
    assert.equal(
      shouldHardKill({ position: 0, maxPosition: 1, totalPnl: -100, maxLossUsd: 100 }).kill,
      true,
    );
    assert.equal(
      shouldHardKill({ position: 0, maxPosition: 1, totalPnl: -10, maxLossUsd: 100 }).kill,
      false,
    );
  });
});
