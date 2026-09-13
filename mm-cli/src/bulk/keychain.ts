import { createRequire } from "node:module";

export interface OrderTypeInput {
  type: string;
  tif?: string;
  isMarket?: boolean;
  triggerPx?: number;
}

export interface OrderInput {
  type: string;
  symbol?: string;
  isBuy?: boolean;
  price?: number;
  size?: number;
  reduceOnly?: boolean;
  iso?: boolean;
  orderType?: OrderTypeInput;
  orderId?: string;
  symbols?: string[];
}

export interface PrepareOptions {
  signatureDomain: string;
  account: string;
  signer?: string;
  nonce?: string;
}

export interface PreparedMessageOutput {
  messageBytes: Buffer;
  actions: string;
  account: string;
  signer: string;
  nonce: string;
  orderId?: string;
  orderIds?: string[];
}

export interface SignedTransactionOutput {
  actions: string;
  nonce: string;
  account: string;
  signer: string;
  signature: string;
  orderId?: string;
  orderIds?: string[];
}

export interface NativeKeypair {
  pubkey: string;
  toBase58(): string;
  toBytes(): Buffer;
}

export interface NativeKeypairCtor {
  new (): NativeKeypair;
  fromBase58(s: string): NativeKeypair;
  fromBytes(bytes: Buffer): NativeKeypair;
}

export interface NativeSigner {
  pubkey: string;
  sign(order: OrderInput, nonce?: string | null): SignedTransactionOutput;
  signGroup(orders: OrderInput[], nonce?: string | null): SignedTransactionOutput;
  signFaucet(nonce?: string | null): SignedTransactionOutput;
  signPrepared(prepared: PreparedMessageOutput): SignedTransactionOutput;
}

export interface NativeSignerCtor {
  new (keypair: NativeKeypair, signatureDomain: string): NativeSigner;
}

export interface BulkKeychain {
  NativeKeypair: NativeKeypairCtor;
  NativeSigner: NativeSignerCtor;
  prepareOrder(order: OrderInput, options: PrepareOptions): PreparedMessageOutput;
  prepareOrderGroup(orders: OrderInput[], options: PrepareOptions): PreparedMessageOutput;
  prepareFaucetRequest(options: PrepareOptions): PreparedMessageOutput;
}

const require = createRequire(import.meta.url);
export const keychain = require("bulk-keychain") as BulkKeychain;
