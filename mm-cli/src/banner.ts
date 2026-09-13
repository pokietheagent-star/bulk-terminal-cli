export const SAFETY_BANNER = [
  "Not financial advice. Market making can lose money.",
  "Dry-run first, then testnet. Mainnet live needs --enable-mainnet and BULK_ALLOW_MAINNET=1.",
].join(" ");

export function printSafetyBanner(write: (line: string) => void = console.log): void {
  write("────────────────────────────────────────────────────────");
  write(SAFETY_BANNER);
  write("────────────────────────────────────────────────────────");
}
