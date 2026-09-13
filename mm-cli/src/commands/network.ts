import { parseNetwork, type Network } from "../config.js";
import { persistPath, readPersisted, writePersisted } from "../persist.js";

export async function networkGet(): Promise<void> {
  const persisted = await readPersisted();
  const env = process.env.BULK_NETWORK;
  console.log(`persisted: ${persisted.network ?? "(none)"}`);
  console.log(`env BULK_NETWORK: ${env ?? "(unset)"}`);
  console.log(`default if unset: testnet`);
  console.log(`file: ${persistPath()}`);
}

export async function networkSet(value: string): Promise<void> {
  const network: Network = parseNetwork(value);
  await writePersisted({ network });
  console.log(`default network set to ${network} (${persistPath()})`);
  console.log("Override anytime with --network testnet|mainnet or BULK_NETWORK.");
}
