import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { keychain } from "./bulk/keychain.js";
import { aloLimit, cancelAll, signQuoteRefresh } from "./bulk/signing.js";

describe("bulk-keychain signing", () => {
  it("signs a cxa + ALO bid/ask batch without posting", () => {
    const keypair = new keychain.NativeKeypair();
    const keys = {
      account: keypair.pubkey,
      agentPubkey: keypair.pubkey,
      signer: new keychain.NativeSigner(keypair, "testnet"),
    };
    const envelope = signQuoteRefresh(keys, "testnet", [
      cancelAll("BTC-USD"),
      aloLimit({ symbol: "BTC-USD", isBuy: true, price: 75_000, size: 0.001 }),
      aloLimit({ symbol: "BTC-USD", isBuy: false, price: 77_000, size: 0.001 }),
    ]);
    assert.equal(envelope.account, keypair.pubkey);
    assert.equal(envelope.signer, keypair.pubkey);
    assert.ok(envelope.signature.length > 20);
    assert.ok(Array.isArray(envelope.actions));
    assert.equal(envelope.actions.length, 3);
  });
});
