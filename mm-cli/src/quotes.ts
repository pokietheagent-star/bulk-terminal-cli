import type { L2Book, MarketInfo, Ticker } from "./bulk/types.js";

export interface QuoteInputs {
  mid: number;
  size: number;
  spreadBps: number;
  inventorySkewStrength: number;
  position: number;
  maxPosition: number;
  tickSize: number;
  lotSize: number;
  minNotional: number;
  pricePrecision: number;
  sizePrecision: number;
}

export interface QuotePlan {
  mid: number;
  reservation: number;
  inventoryRatio: number;
  skewUsd: number;
  bidPx: number;
  askPx: number;
  size: number;
}

export function defaultRequoteThresholdBps(spreadBps: number): number {
  return Math.max(1, spreadBps / 2);
}

export function midFromBook(book: L2Book): number | undefined {
  const bid = book.bids[0]?.px;
  const ask = book.asks[0]?.px;
  if (!isFiniteNumber(bid) || !isFiniteNumber(ask) || bid <= 0 || ask <= 0) {
    return undefined;
  }
  return (bid + ask) / 2;
}

export function midFromTicker(ticker: Ticker): number | undefined {
  for (const value of [ticker.markPrice, ticker.fairBookPx, ticker.lastPrice, ticker.oraclePrice]) {
    if (isFiniteNumber(value) && value > 0) {
      return value;
    }
  }
  return undefined;
}

export function midMovedBps(previous: number, next: number): number {
  if (previous <= 0) {
    return Number.POSITIVE_INFINITY;
  }
  return (Math.abs(next - previous) / previous) * 10_000;
}

export function computeQuotes(input: QuoteInputs): QuotePlan {
  if (!(input.mid > 0)) {
    throw new Error("mid must be positive");
  }
  if (!(input.size > 0)) {
    throw new Error("size must be positive");
  }
  if (!(input.spreadBps > 0)) {
    throw new Error("spreadBps must be positive");
  }
  if (!(input.maxPosition > 0)) {
    throw new Error("maxPosition must be positive");
  }

  const halfSpread = input.mid * (input.spreadBps / 10_000) / 2;
  const inventoryRatio = clamp(input.position / input.maxPosition, -1, 1);
  const skewUsd = inventoryRatio * input.inventorySkewStrength * halfSpread;
  const reservation = input.mid - skewUsd;

  let bidPx = roundToTick(reservation - halfSpread, input.tickSize, "bid", input.pricePrecision);
  let askPx = roundToTick(reservation + halfSpread, input.tickSize, "ask", input.pricePrecision);

  if (bidPx >= askPx) {
    bidPx = roundToTick(input.mid - input.tickSize, input.tickSize, "bid", input.pricePrecision);
    askPx = roundToTick(input.mid + input.tickSize, input.tickSize, "ask", input.pricePrecision);
  }
  if (bidPx >= askPx) {
    throw new Error("could not keep bid below ask after tick rounding");
  }

  const size = roundToLot(input.size, input.lotSize, input.sizePrecision);
  if (size <= 0) {
    throw new Error("size rounded to zero against lotSize");
  }
  const minPx = Math.min(bidPx, askPx);
  if (minPx * size < input.minNotional) {
    throw new Error(
      `size ${size} at ${minPx} is below minNotional ${input.minNotional}; increase --size`,
    );
  }

  return { mid: input.mid, reservation, inventoryRatio, skewUsd, bidPx, askPx, size };
}

export function marketQuoteInputs(market: MarketInfo): Pick<
  QuoteInputs,
  "tickSize" | "lotSize" | "minNotional" | "pricePrecision" | "sizePrecision"
> {
  return {
    tickSize: market.tickSize,
    lotSize: market.lotSize,
    minNotional: market.minNotional,
    pricePrecision: market.pricePrecision,
    sizePrecision: market.sizePrecision,
  };
}

function roundToTick(
  price: number,
  tickSize: number,
  side: "bid" | "ask",
  precision: number,
): number {
  const tick = tickSize > 0 ? tickSize : 10 ** -Math.max(precision, 0);
  const ticks = price / tick;
  const rounded = side === "bid" ? Math.floor(ticks) * tick : Math.ceil(ticks) * tick;
  return roundPrecision(Math.max(rounded, tick), precision);
}

function roundToLot(size: number, lotSize: number, precision: number): number {
  const lot = lotSize > 0 ? lotSize : 10 ** -Math.max(precision, 0);
  return roundPrecision(Math.floor(size / lot) * lot, precision);
}

function roundPrecision(value: number, precision: number): number {
  const digits = Math.max(0, Math.min(12, Math.floor(precision)));
  return Number(value.toFixed(digits));
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}
