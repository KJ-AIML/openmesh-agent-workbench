// Model color palette matching the source dashboard
export const MODEL_COLORS: Record<string, string> = {
  "glm-5.2": "#4A90E2",
  "deepseek-v4-flash": "#50C878",
  "minimax-m3": "#9370DB",
  "nex-n2-pro": "#FF6B6B",
  "nemotron-3-ultra": "#FFA500",
  "step-3.7-flash": "#20B2AA",
  "qwen3.6-27b": "#FF69B4",
};

export const MODELS = Object.keys(MODEL_COLORS);

// Generate ~30 days of daily usage per model (deterministic seed-like)
function seededRandom(seed: number) {
  const x = Math.sin(seed) * 10000;
  return x - Math.floor(x);
}

export type DailyUsage = {
  date: string; // ISO
  label: string; // "MAY 30", "JUN 26" etc
  short: string; // "May 30"
  values: Record<string, number>; // model -> req count
  total: number;
  totalTokens: number;
};

function formatDate(d: Date): { label: string; short: string; iso: string } {
  const months = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
  const monthsShort = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  const label = `${months[d.getMonth()]} ${d.getDate()}`;
  const short = `${monthsShort[d.getMonth()]} ${d.getDate()}`;
  const iso = d.toISOString().slice(0, 10);
  return { label, short, iso };
}

export function buildDailyUsage(): DailyUsage[] {
  const out: DailyUsage[] = [];
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  // 30 days ending today
  for (let i = 29; i >= 0; i--) {
    const d = new Date(today);
    d.setDate(today.getDate() - i);
    const { label, short, iso } = formatDate(d);

    // generate per-model req counts
    const values: Record<string, number> = {};
    let total = 0;
    MODELS.forEach((m, idx) => {
      const seed = (i + 1) * 7 + idx * 13 + 1;
      // glm-5.2 dominates, then deepseek, then minimax, etc.
      const baseWeights = [0.45, 0.25, 0.18, 0.05, 0.04, 0.02, 0.01];
      const dayVariance = 0.6 + seededRandom(seed) * 0.8; // 0.6 - 1.4
      const baseDay = 700 * dayVariance;
      const v = Math.max(0, Math.round(baseDay * baseWeights[idx]));
      values[m] = v;
      total += v;
    });

    const totalTokens = total * (70_000 + Math.round(seededRandom(i + 100) * 30_000));
    out.push({ date: iso, label, short, values, total, totalTokens });
  }
  return out;
}

export type ModelPerf = {
  model: string;
  requests: number; // raw
  requestsLabel: string; // "2.5K"
  tokens: number; // raw
  tokensLabel: string; // "193.71M"
  latency: number; // seconds
  ttft: number; // seconds
  prefill: number; // per second
  gen: number; // per second
};

export const MODEL_PERFORMANCE: ModelPerf[] = [
  {
    model: "glm-5.2",
    requests: 2500,
    requestsLabel: "2.5K",
    tokens: 193_710_000,
    tokensLabel: "193.71M",
    latency: 18.1,
    ttft: 2.9,
    prefill: 78_000,
    gen: 118,
  },
  {
    model: "deepseek-v4-flash",
    requests: 2800,
    requestsLabel: "2.8K",
    tokens: 167_510_000,
    tokensLabel: "167.51M",
    latency: 16.2,
    ttft: 3.1,
    prefill: 70_000,
    gen: 187,
  },
  {
    model: "minimax-m3",
    requests: 2000,
    requestsLabel: "2.0K",
    tokens: 142_740_000,
    tokensLabel: "142.74M",
    latency: 6.3,
    ttft: 1.3,
    prefill: 0,
    gen: 69,
  },
  {
    model: "nex-n2-pro",
    requests: 507,
    requestsLabel: "507",
    tokens: 52_710_000,
    tokensLabel: "52.71M",
    latency: 10.9,
    ttft: 1.7,
    prefill: 84_000,
    gen: 160,
  },
  {
    model: "nemotron-3-ultra",
    requests: 535,
    requestsLabel: "535",
    tokens: 33_180_000,
    tokensLabel: "33.18M",
    latency: 30.5,
    ttft: 22.8,
    prefill: 62_000,
    gen: 166,
  },
  {
    model: "step-3.7-flash",
    requests: 670,
    requestsLabel: "670",
    tokens: 31_670_000,
    tokensLabel: "31.67M",
    latency: 5.9,
    ttft: 1.2,
    prefill: 58_000,
    gen: 0,
  },
];

// Sidebar nav structure
export const SIDEBAR_NAV = [
  { label: "Home", icon: "home" },
  {
    section: "Workspace",
    items: [
      { label: "Status", icon: "circle" },
      { label: "Usage", icon: "chart", active: true },
      { label: "Models", icon: "folder" },
      { label: "Server", icon: "globe" },
    ],
  },
  {
    section: "Projects",
    items: [
      { label: "plugins" },
      { label: "personal" },
      { label: "vllm-studio" },
      { label: "deepseek-flas…" },
      { label: "parchi" },
      { label: "lambda" },
      { label: "ai" },
    ],
  },
  {
    section: "Chats",
    items: [],
  },
];

export const PROJECT_LIST = [
  "plugins",
  "personal",
  "vllm-studio",
  "deepseek-flas…",
  "parchi",
  "lambda",
  "ai",
];

export const TOP_NAV = ["SOURCE", "Provider", "Pi sessions"];
