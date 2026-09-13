import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  computeQuotes,
  defaultRequoteThresholdBps,
  midFromBook,
  midFromTicker,
  midMovedBps,
} from "./quotes.js";

describe("defaultRequoteThresholdBps", () => {
  it("is half of spread, minimum 1", () => {
    assert.equal(defaultRequoteThresholdBps(10), 5);
    assert.equal(defaultRequoteThresholdBps(1), 1);
    assert.equal(defaultRequoteThresholdBps(0.5), 1);
  });
});

describe("computeQuotes", () => {
  const base = {
    mid: 100_000,
    size: 0.002,
    spreadBps: 10,
    inventorySkewStrength: 1,
    maxPosition: 1,
    tickSize: 0.001,
    lotSize: 0.000001,
    minNotional: 1,
    pricePrecision: 3,
    sizePrecision: 6,
  };

  it("centers ALO bid/ask around mid when flat", () => {
    const quotes = computeQuotes({ ...base, position: 0 });
    assert.equal(quotes.bidPx, 99_950);
    assert.equal(quotes.askPx, 100_050);
    assert.equal(quotes.size, 0.002);
  });

  it("leans down when long so inventory is reduced", () => {
    const quotes = computeQuotes({ ...base, position: 1 });
    assert.ok(quotes.bidPx < 99_950);
    assert.ok(quotes.askPx < 100_050);
    assert.ok(quotes.reservation < quotes.mid);
  });

  it("leans up when short so inventory is reduced", () => {
    const quotes = computeQuotes({ ...base, position: -1 });
    assert.ok(quotes.bidPx > 99_950);
    assert.ok(quotes.askPx > 100_050);
    assert.ok(quotes.reservation > quotes.mid);
  });
});

describe("mids", () => {
  it("uses L2 mid when both sides exist", () => {
    assert.equal(
      midFromBook({
        symbol: "BTC-USD",
        bids: [{ px: 100, sz: 1 }],
        asks: [{ px: 102, sz: 1 }],
      }),
      101,
    );
  });

  it("falls back through ticker fields", () => {
    assert.equal(midFromTicker({ symbol: "BTC-USD", lastPrice: 50, markPrice: 51 }), 51);
  });

  it("measures mid move in bps", () => {
    assert.ok(Math.abs(midMovedBps(100, 100.1) - 10) < 1e-9);
  });
});
