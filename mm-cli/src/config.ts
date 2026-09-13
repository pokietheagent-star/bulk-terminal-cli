import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { defaultRequoteThresholdBps } from "./quotes.js";

export type Network = "testnet" | "mainnet";
export type Mode = "dry-run" | "live";

export interface RunConfig {
  market: string;
  size: number;
  spreadBps: number;
  requoteThresholdBps: number;
  requoteThresholdDefaulted: boolean;
  inventorySkewStrength: number;
  maxPosition: number;
  maxLossUsd: number;
  mode: Mode;
  network: Network;
  enableMainnet: boolean;
  intervalMs: number;
  ticks?: number;
  simulatePosition?: number;
  configPath?: string;
}

export interface CliOverrides {
  market?: string;
  size?: number;
  spreadBps?: number;
  requoteThresholdBps?: number;
  inventorySkewStrength?: number;
  maxPosition?: number;
  maxLossUsd?: number;
  mode?: Mode;
  network?: Network;
  enableMainnet?: boolean;
  intervalMs?: number;
  ticks?: number;
  simulatePosition?: number;
  config?: string;
}

export interface FileConfig {
  market?: string;
  size?: number;
  spreadBps?: number;
  requoteThresholdBps?: number;
  inventorySkewStrength?: number;
  maxPosition?: number;
  maxLossUsd?: number;
  mode?: Mode;
  network?: Network;
  intervalMs?: number;
}

const DEFAULTS = {
  market: "BTC-USD",
  size: 0.001,
  spreadBps: 10,
  inventorySkewStrength: 0.5,
  maxPosition: 0.01,
  maxLossUsd: 100,
  mode: "dry-run" as Mode,
  network: "testnet" as Network,
  intervalMs: 2000,
};

export function resolveRunConfig(
  overrides: CliOverrides,
  persistedNetwork?: Network,
): RunConfig {
  const file = loadOptionalConfig(overrides.config);
  const spreadBps = requirePositive(
    overrides.spreadBps ?? file.spreadBps ?? envNumber("BULK_SPREAD_BPS") ?? DEFAULTS.spreadBps,
    "spreadBps",
  );
  const requoteExplicit =
    overrides.requoteThresholdBps ?? file.requoteThresholdBps ?? envNumber("BULK_REQUOTE_THRESHOLD_BPS");
  const requoteThresholdBps = requoteExplicit ?? defaultRequoteThresholdBps(spreadBps);
  if (!(requoteThresholdBps > 0)) {
    throw new Error("requoteThresholdBps must be positive");
  }

  const network = parseNetwork(
    overrides.network
      ?? file.network
      ?? process.env.BULK_NETWORK
      ?? persistedNetwork
      ?? DEFAULTS.network,
  );

  return {
    market: overrides.market ?? file.market ?? process.env.BULK_MARKET ?? DEFAULTS.market,
    size: requirePositive(overrides.size ?? file.size ?? envNumber("BULK_SIZE") ?? DEFAULTS.size, "size"),
    spreadBps,
    requoteThresholdBps,
    requoteThresholdDefaulted: requoteExplicit === undefined,
    inventorySkewStrength: requireNonNegative(
      overrides.inventorySkewStrength
        ?? file.inventorySkewStrength
        ?? envNumber("BULK_INVENTORY_SKEW_STRENGTH")
        ?? DEFAULTS.inventorySkewStrength,
      "inventorySkewStrength",
    ),
    maxPosition: requirePositive(
      overrides.maxPosition ?? file.maxPosition ?? envNumber("BULK_MAX_POSITION") ?? DEFAULTS.maxPosition,
      "maxPosition",
    ),
    maxLossUsd: requirePositive(
      overrides.maxLossUsd ?? file.maxLossUsd ?? envNumber("BULK_MAX_LOSS_USD") ?? DEFAULTS.maxLossUsd,
      "maxLossUsd",
    ),
    mode: parseMode(overrides.mode ?? file.mode ?? process.env.BULK_MODE ?? DEFAULTS.mode),
    network,
    enableMainnet: Boolean(overrides.enableMainnet),
    intervalMs: requirePositive(
      overrides.intervalMs ?? file.intervalMs ?? envNumber("BULK_INTERVAL_MS") ?? DEFAULTS.intervalMs,
      "intervalMs",
    ),
    ticks: overrides.ticks,
    simulatePosition: overrides.simulatePosition,
    configPath: overrides.config ? resolve(overrides.config) : undefined,
  };
}

export function parseNetwork(value: string): Network {
  if (value === "testnet" || value === "mainnet") {
    return value;
  }
  throw new Error(`network must be testnet or mainnet, got ${value}`);
}

export function parseMode(value: string): Mode {
  if (value === "dry-run" || value === "live") {
    return value;
  }
  throw new Error(`mode must be dry-run or live, got ${value}`);
}

function loadOptionalConfig(path?: string): FileConfig {
  if (!path) {
    return {};
  }
  const raw = readFileSync(resolve(path), "utf8");
  return JSON.parse(raw) as FileConfig;
}

function envNumber(name: string): number | undefined {
  const raw = process.env[name];
  if (raw === undefined || raw === "") {
    return undefined;
  }
  const value = Number(raw);
  if (!Number.isFinite(value)) {
    throw new Error(`${name} must be a number`);
  }
  return value;
}

function requirePositive(value: number, name: string): number {
  if (!(value > 0) || !Number.isFinite(value)) {
    throw new Error(`${name} must be a positive number`);
  }
  return value;
}

function requireNonNegative(value: number, name: string): number {
  if (!(value >= 0) || !Number.isFinite(value)) {
    throw new Error(`${name} must be >= 0`);
  }
  return value;
}
