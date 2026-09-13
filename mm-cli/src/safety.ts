import type { Mode, Network } from "./config.js";

export class MainnetLiveBlockedError extends Error {
  constructor() {
    super(
      [
        "Refused: mainnet live trading is blocked.",
        "It is not reachable by accident. Enable it only with BOTH:",
        "  1. --enable-mainnet",
        "  2. env BULK_ALLOW_MAINNET=1",
        "Dry-run works on mainnet without this gate (still places nothing).",
      ].join("\n"),
    );
    this.name = "MainnetLiveBlockedError";
  }
}

export function assertMainnetLiveAllowed(input: {
  network: Network;
  mode: Mode;
  enableMainnet: boolean;
  allowMainnetEnv?: string;
}): void {
  if (input.network !== "mainnet" || input.mode !== "live") {
    return;
  }
  const envOk = input.allowMainnetEnv === "1";
  if (!input.enableMainnet || !envOk) {
    throw new MainnetLiveBlockedError();
  }
}

export function shouldHardKill(input: {
  position: number;
  maxPosition: number;
  totalPnl: number;
  maxLossUsd: number;
}): { kill: boolean; reason?: string } {
  if (Math.abs(input.position) >= input.maxPosition) {
    return {
      kill: true,
      reason: `max position ${input.maxPosition} reached (position=${input.position})`,
    };
  }
  if (input.totalPnl <= -input.maxLossUsd) {
    return {
      kill: true,
      reason: `max loss USD ${input.maxLossUsd} reached (pnl=${input.totalPnl})`,
    };
  }
  return { kill: false };
}
