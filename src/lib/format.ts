// Number formatting helpers. Pure functions, no data.

export function formatTokens(n: number): string {
  if (!n) return "0";
  if (n >= 1_000_000_000) return (n / 1_000_000_000).toFixed(2) + "B";
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + "M";
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "K";
  return n.toString();
}

export function formatRequests(n: number): string {
  if (!n) return "0";
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
  if (n >= 1_000) return (n / 1_000).toFixed(1) + "K";
  return n.toString();
}

export function formatNumber(n: number): string {
  return n.toLocaleString();
}

export function formatLatency(seconds: number): string {
  if (!seconds) return "—";
  if (seconds < 1) return `${Math.round(seconds * 1000)}ms`;
  return `${seconds.toFixed(1)}s`;
}

export function formatSpeed(prefill: number, gen: number): string {
  const parts: string[] = [];
  if (prefill > 0) parts.push(`${Math.round(prefill / 1000)}K prefill/s`);
  if (gen > 0) parts.push(`${gen} gen/s`);
  return parts.join(" · ") || "—";
}

export function formatPercent(value: number, total: number): string {
  if (!total) return "0.0%";
  return ((value / total) * 100).toFixed(1) + "%";
}
