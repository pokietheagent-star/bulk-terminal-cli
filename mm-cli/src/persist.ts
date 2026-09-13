import { mkdir, readFile, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import type { Network } from "./config.js";

export interface PersistedSettings {
  network?: Network;
}

export function persistPath(): string {
  return join(homedir(), ".config", "bulk-mm", "config.json");
}

export async function readPersisted(): Promise<PersistedSettings> {
  try {
    const raw = await readFile(persistPath(), "utf8");
    const parsed = JSON.parse(raw) as PersistedSettings;
    if (parsed.network === "testnet" || parsed.network === "mainnet") {
      return { network: parsed.network };
    }
    return {};
  } catch {
    return {};
  }
}

export async function writePersisted(settings: PersistedSettings): Promise<void> {
  const path = persistPath();
  await mkdir(dirname(path), { recursive: true });
  const current = await readPersisted();
  const next = { ...current, ...settings };
  await writeFile(path, `${JSON.stringify(next, null, 2)}\n`, "utf8");
}
