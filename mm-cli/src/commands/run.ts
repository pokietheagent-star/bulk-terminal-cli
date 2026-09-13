import type { CliOverrides } from "../config.js";
import { resolveRunConfig } from "../config.js";
import { runEngine } from "../engine.js";
import { readPersisted } from "../persist.js";

export async function runCommand(overrides: CliOverrides): Promise<void> {
  const persisted = await readPersisted();
  const config = resolveRunConfig(overrides, persisted.network);
  await runEngine(config);
}
