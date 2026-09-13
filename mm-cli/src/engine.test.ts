import assert from "node:assert/strict";
import { describe, it } from "node:test";
import type { BulkClient } from "./bulk/client.js";
import type { AccountSnapshot, L2Book, MarketInfo, SignedEnvelope, Ticker } from "./bulk/types.js";
import { resolveRunConfig } from "./config.js";
import { runEngine } from "./engine.js";

class MockClient {
  submitted: SignedEnvelope[] = [];

  async exchangeInfo(): Promise<MarketInfo[]> {
    return [this.market()];
  }

  async market(): Promise<MarketInfo> {
    return {
      symbol: "BTC-USD",
      status: "TRADING",
      tickSize: 0.001,
      lotSize: 0.000001,
      minNotional: 1,
      pricePrecision: 3,
      sizePrecision: 6,
    };
  }

  async ticker(): Promise<Ticker> {
    return { symbol: "BTC-USD", markPrice: 76_000 };
  }

  async l2book(): Promise<L2Book> {
    return {
      symbol: "BTC-USD",
      bids: [{ px: 75_999, sz: 1 }],
      asks: [{ px: 76_001, sz: 1 }],
    };
  }

  async account(): Promise<AccountSnapshot> {
    return { position: 0, realizedPnl: 0, unrealizedPnl: 0, totalPnl: 0 };
  }

  async submit(envelope: SignedEnvelope): Promise<unknown> {
    this.submitted.push(envelope);
    return { status: "ok" };
  }
}

describe("runEngine dry-run", () => {
  it("prints ALO bid/ask and never POST /order", async () => {
    const client = new MockClient();
    const lines: string[] = [];
    await runEngine(resolveRunConfig({ mode: "dry-run", ticks: 1, intervalMs: 1 }), {
      client: client as unknown as BulkClient,
      log: (line) => lines.push(line),
      sleep: async () => undefined,
    });

    const joined = lines.join("\n");
    assert.match(joined, /\[dry-run\]/);
    assert.match(joined, /ALO bid=/);
    assert.match(joined, /ask=/);
    assert.match(joined, /no orders posted/);
    assert.match(joined, /would cancel-all \(cxa BTC-USD\)/);
    assert.equal(client.submitted.length, 0);
  });
});
