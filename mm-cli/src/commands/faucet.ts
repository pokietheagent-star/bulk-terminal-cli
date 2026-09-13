import { printSafetyBanner } from "../banner.js";
import { BulkClient, NETWORKS } from "../bulk/client.js";
import { loadTradingKeys, signFaucet } from "../bulk/signing.js";
import { parseNetwork } from "../config.js";
import { readPersisted } from "../persist.js";

export async function faucetCommand(networkFlag?: string): Promise<void> {
  printSafetyBanner();
  const persisted = await readPersisted();
  const network = parseNetwork(networkFlag ?? process.env.BULK_NETWORK ?? persisted.network ?? "testnet");
  if (network !== "testnet") {
    throw new Error("faucet is testnet-only (~10k / 24h). Use --network testnet.");
  }
  const keys = loadTradingKeys(network);
  const envelope = signFaucet(keys, network);
  const client = new BulkClient(NETWORKS.testnet.http);
  console.log(`requesting testnet faucet for account=${keys.account} (signed; ~10k / 24h)`);
  const response = await client.submit(envelope);
  console.log(JSON.stringify(response, null, 2));
}
