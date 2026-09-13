import type { AccountSnapshot, L2Book, MarketInfo, SignedEnvelope, Ticker } from "./types.js";

export const NETWORKS = {
  testnet: {
    http: "https://exchange-api.bulk.trade/api/v1",
    ws: "wss://exchange-ws1.bulk.trade",
    domain: "testnet" as const,
  },
  mainnet: {
    http: "https://mainnet-api1.bulk.trade/api/v1",
    ws: "wss://mainnet-ws1.bulk.trade",
    domain: "mainnet" as const,
  },
};

export class BulkClient {
  constructor(
    private readonly baseUrl: string,
    private readonly fetchImpl: typeof fetch = fetch,
  ) {}

  async exchangeInfo(): Promise<MarketInfo[]> {
    const raw = await this.getJson("/exchangeInfo");
    if (!Array.isArray(raw)) {
      throw new Error("exchangeInfo: expected an array");
    }
    return raw.map(parseMarket);
  }

  async market(symbol: string): Promise<MarketInfo> {
    const markets = await this.exchangeInfo();
    const found = markets.find((m) => m.symbol === symbol);
    if (!found) {
      throw new Error(`market ${symbol} not found on this network`);
    }
    return found;
  }

  async ticker(symbol: string): Promise<Ticker> {
    const raw = await this.getJson(`/ticker/${encodeURIComponent(symbol)}`);
    return {
      symbol: String(asRecord(raw).symbol ?? symbol),
      lastPrice: num(asRecord(raw).lastPrice),
      markPrice: num(asRecord(raw).markPrice),
      oraclePrice: num(asRecord(raw).oraclePrice),
      fairBookPx: num(asRecord(raw).fairBookPx),
    };
  }

  async l2book(symbol: string, nlevels = 5): Promise<L2Book> {
    const query = new URLSearchParams({
      type: "l2book",
      coin: symbol,
      nlevels: String(nlevels),
    });
    const raw = asRecord(await this.getJson(`/l2book?${query}`));
    const levels = raw.levels;
    if (!Array.isArray(levels) || levels.length < 2) {
      throw new Error("l2book: expected levels [bids, asks]");
    }
    return {
      symbol: String(raw.symbol ?? symbol),
      bids: parseLevels(levels[0]),
      asks: parseLevels(levels[1]),
    };
  }

  async account(user: string, symbol?: string): Promise<AccountSnapshot> {
    const raw = await this.postJson("/account", { type: "fullAccount", user });
    return parseAccount(raw, user, symbol);
  }

  async submit(envelope: SignedEnvelope): Promise<unknown> {
    return this.postJson("/order", {
      actions: envelope.actions,
      nonce: envelope.nonce,
      account: envelope.account,
      signer: envelope.signer,
      signature: envelope.signature,
    });
  }

  private async getJson(path: string): Promise<unknown> {
    const res = await this.fetchImpl(`${this.baseUrl}${path}`);
    return readJson(res, `GET ${path}`);
  }

  private async postJson(path: string, body: unknown): Promise<unknown> {
    const res = await this.fetchImpl(`${this.baseUrl}${path}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    return readJson(res, `POST ${path}`);
  }
}

async function readJson(res: Response, label: string): Promise<unknown> {
  const text = await res.text();
  if (!res.ok) {
    throw new Error(`${label} failed ${res.status}: ${text.slice(0, 400)}`);
  }
  try {
    return JSON.parse(text) as unknown;
  } catch {
    throw new Error(`${label} returned non-JSON`);
  }
}

function parseMarket(raw: unknown): MarketInfo {
  const row = asRecord(raw);
  return {
    symbol: String(row.symbol),
    status: String(row.status ?? ""),
    tickSize: requireNum(row.tickSize, "tickSize"),
    lotSize: requireNum(row.lotSize, "lotSize"),
    minNotional: requireNum(row.minNotional, "minNotional"),
    pricePrecision: Number(row.pricePrecision ?? 8),
    sizePrecision: Number(row.sizePrecision ?? 8),
  };
}

function parseLevels(raw: unknown): L2Book["bids"] {
  if (!Array.isArray(raw)) {
    return [];
  }
  return raw.flatMap((level) => {
    const row = asRecord(level);
    const px = num(row.px);
    const sz = num(row.sz);
    if (px === undefined || sz === undefined) {
      return [];
    }
    return [{ px, sz, n: num(row.n) }];
  });
}

function parseAccount(raw: unknown, user: string, symbol?: string): AccountSnapshot {
  const rows = Array.isArray(raw) ? raw : [raw];
  for (const row of rows) {
    const wrapped = asRecord(row);
    const account = asRecord(wrapped.fullAccount ?? wrapped);
    const margin = asRecord(account.margin ?? {});
    const realizedPnl = num(margin.realizedPnl) ?? 0;
    const unrealizedPnl = num(margin.unrealizedPnl) ?? 0;
    const positions = Array.isArray(account.positions) ? account.positions : [];
    const match = symbol
      ? positions.find((item) => String(asRecord(item).symbol ?? "") === symbol)
      : positions[0];
    const position = match ? signedSize(asRecord(match)) : 0;
    return {
      position,
      realizedPnl,
      unrealizedPnl,
      totalPnl: realizedPnl + unrealizedPnl,
    };
  }
  throw new Error(`account snapshot empty for ${user}`);
}

function signedSize(row: Record<string, unknown>): number {
  const size = num(row.size ?? row.sz) ?? 0;
  const side = String(row.side ?? row.d ?? "").toLowerCase();
  if (side === "short" || side === "sell") {
    return -Math.abs(size);
  }
  return size;
}

function asRecord(value: unknown): Record<string, unknown> {
  if (value && typeof value === "object" && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return {};
}

function num(value: unknown): number | undefined {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === "string" && value !== "") {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) {
      return parsed;
    }
  }
  return undefined;
}

function requireNum(value: unknown, name: string): number {
  const parsed = num(value);
  if (parsed === undefined) {
    throw new Error(`missing numeric field ${name}`);
  }
  return parsed;
}
