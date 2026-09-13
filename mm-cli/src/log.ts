export function redactSecrets(text: string): string {
  let next = text;
  for (const name of ["BULK_AGENT_SECRET", "BULK_AGENT_SECRET_PATH"]) {
    const value = process.env[name];
    if (value && value.length > 4) {
      next = next.split(value).join(`${value.slice(0, 4)}…`);
    }
  }
  return next;
}

export function formatUsd(value: number, digits = 2): string {
  return value.toFixed(digits);
}

export function formatPx(value: number): string {
  return String(value);
}
