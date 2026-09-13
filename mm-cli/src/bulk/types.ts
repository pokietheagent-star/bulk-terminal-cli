export interface MarketInfo {
  symbol: string;
  status: string;
  tickSize: number;
  lotSize: number;
  minNotional: number;
  pricePrecision: number;
  sizePrecision: number;
}

export interface Ticker {
  symbol: string;
  lastPrice?: number;
  markPrice?: number;
  oraclePrice?: number;
  fairBookPx?: number;
}

export interface BookLevel {
  px: number;
  sz: number;
  n?: number;
}

export interface L2Book {
  symbol: string;
  bids: BookLevel[];
  asks: BookLevel[];
}

export interface AccountSnapshot {
  position: number;
  realizedPnl: number;
  unrealizedPnl: number;
  totalPnl: number;
}

export interface SignedEnvelope {
  actions: unknown[];
  nonce: string;
  account: string;
  signer: string;
  signature: string;
  orderId?: string;
  orderIds?: string[];
}
