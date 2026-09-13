import { readFileSync } from "node:fs";
import type { Network } from "../config.js";
import {
  keychain,
  type NativeSigner,
  type OrderInput,
  type SignedTransactionOutput,
} from "./keychain.js";
import type { SignedEnvelope } from "./types.js";

const {
  NativeKeypair,
  NativeSigner: Signer,
  prepareFaucetRequest,
  prepareOrder,
  prepareOrderGroup,
} = keychain;

export interface TradingKeys {
  account: string;
  agentPubkey: string;
  signer: NativeSigner;
}

export function loadTradingKeys(network: Network): TradingKeys {
  const account = requiredEnv("BULK_ACCOUNT");
  const secret = readAgentSecret();
  const keypair = NativeKeypair.fromBase58(secret);
  const agentPubkey = process.env.BULK_AGENT_PUBKEY?.trim() || keypair.pubkey;
  if (process.env.BULK_AGENT_PUBKEY && process.env.BULK_AGENT_PUBKEY !== keypair.pubkey) {
    throw new Error("BULK_AGENT_PUBKEY does not match the secret key");
  }
  return {
    account,
    agentPubkey,
    signer: new Signer(keypair, network),
  };
}

export function aloLimit(input: {
  symbol: string;
  isBuy: boolean;
  price: number;
  size: number;
}): OrderInput {
  return {
    type: "order",
    symbol: input.symbol,
    isBuy: input.isBuy,
    price: input.price,
    size: input.size,
    reduceOnly: false,
    iso: false,
    orderType: { type: "limit", tif: "ALO" },
  };
}

export function cancelAll(symbol: string): OrderInput {
  return { type: "cancelAll", symbols: [symbol] };
}

export function signQuoteRefresh(
  keys: TradingKeys,
  network: Network,
  actions: OrderInput[],
): SignedEnvelope {
  return toEnvelope(signForAccount(keys, network, actions));
}

export function signCancelAll(keys: TradingKeys, network: Network, symbol: string): SignedEnvelope {
  return toEnvelope(signForAccount(keys, network, [cancelAll(symbol)]));
}

export function signFaucet(keys: TradingKeys, network: Network): SignedEnvelope {
  if (keys.account === keys.agentPubkey) {
    return toEnvelope(keys.signer.signFaucet());
  }
  const prepared = prepareFaucetRequest({
    signatureDomain: network,
    account: keys.account,
    signer: keys.agentPubkey,
  });
  return toEnvelope(keys.signer.signPrepared(prepared));
}

function signForAccount(
  keys: TradingKeys,
  network: Network,
  actions: OrderInput[],
): SignedTransactionOutput {
  if (keys.account === keys.agentPubkey) {
    return actions.length === 1
      ? keys.signer.sign(actions[0]!)
      : keys.signer.signGroup(actions);
  }
  const options = {
    signatureDomain: network,
    account: keys.account,
    signer: keys.agentPubkey,
  };
  const prepared = actions.length === 1
    ? prepareOrder(actions[0]!, options)
    : prepareOrderGroup(actions, options);
  return keys.signer.signPrepared(prepared);
}

function toEnvelope(signed: SignedTransactionOutput): SignedEnvelope {
  return {
    actions: JSON.parse(signed.actions) as unknown[],
    nonce: signed.nonce,
    account: signed.account,
    signer: signed.signer,
    signature: signed.signature,
    orderId: signed.orderId,
    orderIds: signed.orderIds,
  };
}

function readAgentSecret(): string {
  const path = process.env.BULK_AGENT_SECRET_PATH?.trim();
  if (path) {
    const raw = readFileSync(path, "utf8").trim();
    return normalizeSecret(raw);
  }
  const secret = process.env.BULK_AGENT_SECRET?.trim();
  if (!secret) {
    throw new Error("set BULK_AGENT_SECRET or BULK_AGENT_SECRET_PATH for live / faucet");
  }
  return normalizeSecret(secret);
}

function normalizeSecret(raw: string): string {
  if (raw.startsWith("[")) {
    const bytes = Uint8Array.from(JSON.parse(raw) as number[]);
    return NativeKeypair.fromBytes(Buffer.from(bytes)).toBase58();
  }
  return raw;
}

function requiredEnv(name: string): string {
  const value = process.env[name]?.trim();
  if (!value) {
    throw new Error(`${name} is required for live trading and faucet`);
  }
  return value;
}
