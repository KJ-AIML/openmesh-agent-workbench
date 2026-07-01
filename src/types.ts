// Type definitions for the usage dashboard.
// No mock data lives here — only shapes and an EMPTY_USAGE constant
// representing a fresh account with zero activity.

export type ModelMetrics = {
  model: string;
  requests: number;
  tokens: number;
  latency: number; // seconds
  ttft: number; // seconds
  prefill: number; // tokens/s
  gen: number; // tokens/s
};

export type DailyBucket = {
  date: string; // ISO date
  label: string; // "Jun 26"
  values: Record<string, number>; // model -> requests
  total: number;
  totalTokens: number;
};

export type UsageSummary = {
  totalTokens: number;
  totalRequests: number;
  sessions: number;
  users: number;
  promptTokens: number;
  completionTokens: number;
  requests24h: number;
  requestsLastHour: number;
  avgTokens: number;
  avgTokensIn: number;
  avgTokensOut: number;
};

export type DashboardData = {
  summary: UsageSummary;
  daily: DailyBucket[];
  totals: {
    totalTokens: number;
    totalRequests: number;
    peakBucket: number;
  };
  models: ModelMetrics[];
};

export const EMPTY_USAGE: DashboardData = {
  summary: {
    totalTokens: 0,
    totalRequests: 0,
    sessions: 0,
    users: 0,
    promptTokens: 0,
    completionTokens: 0,
    requests24h: 0,
    requestsLastHour: 0,
    avgTokens: 0,
    avgTokensIn: 0,
    avgTokensOut: 0,
  },
  daily: [],
  totals: {
    totalTokens: 0,
    totalRequests: 0,
    peakBucket: 0,
  },
  models: [],
};

// Color palette for known model names. Used both by the chart and table.
export const MODEL_COLORS: Record<string, string> = {
  "glm-5.2": "#4A90E2",
  "deepseek-v4-flash": "#50C878",
  "minimax-m3": "#9370DB",
  "nex-n2-pro": "#FF6B6B",
  "nemotron-3-ultra": "#FFA500",
  "step-3.7-flash": "#20B2AA",
  "qwen3.6-27b": "#FF69B4",
};

// Fallback color cycle for unknown models
export const FALLBACK_COLORS = [
  "#4A90E2",
  "#50C878",
  "#9370DB",
  "#FF6B6B",
  "#FFA500",
  "#20B2AA",
  "#FF69B4",
  "#F4D03F",
  "#5DADE2",
  "#48C9B0",
];

export function colorForModel(model: string, idx: number): string {
  return MODEL_COLORS[model] ?? FALLBACK_COLORS[idx % FALLBACK_COLORS.length];
}
