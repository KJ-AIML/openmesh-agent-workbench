<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import {
  AlertCircle,
  BarChart3,
  ChevronLeft,
  ChevronRight,
  CircleDashed,
  RefreshCw,
} from "lucide-vue-next";
import {
  formatCost,
  formatDuration,
  formatTokenCount,
  getDefaultDateRange,
  getUsageRequestLogs,
  getUsageSummary,
  getUsageTimeseries,
  type RequestLogEntry,
  type UsageBucket,
  type UsageSummary,
} from "../lib/usageClient";

type DatePreset = "24h" | "7d" | "30d" | "custom";
type BreakdownTab = "provider" | "model" | "account";

const LOGS_PER_PAGE = 50;

const datePreset = ref<DatePreset>("24h");
const customStart = ref("");
const customEnd = ref("");
const range = ref(getDefaultDateRange());

const summary = ref<UsageSummary | null>(null);
const timeseries = ref<UsageBucket[]>([]);
const logs = ref<RequestLogEntry[]>([]);
const logsPage = ref(0);
const logsHasMore = ref(false);

const breakdownTab = ref<BreakdownTab>("provider");
const loading = ref(true);
const logsLoading = ref(false);
const error = ref("");

const startLabel = computed(() => new Date(range.value.start).toLocaleString());
const endLabel = computed(() => new Date(range.value.end).toLocaleString());

const chartMaxTokens = computed(() => {
  if (timeseries.value.length === 0) return 1;
  return Math.max(1, ...timeseries.value.map((b) => b.totalTokens));
});

const chartPath = computed(() => {
  const buckets = timeseries.value;
  if (buckets.length === 0) return "";
  const width = 600;
  const height = 160;
  const step = buckets.length > 1 ? width / (buckets.length - 1) : width;
  const max = chartMaxTokens.value;
  return buckets
    .map((bucket, i) => {
      const x = i * step;
      const y = height - (bucket.totalTokens / max) * height;
      return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
});

const chartAreaPath = computed(() => {
  const buckets = timeseries.value;
  if (buckets.length === 0) return "";
  const width = 600;
  const height = 160;
  const step = buckets.length > 1 ? width / (buckets.length - 1) : width;
  const max = chartMaxTokens.value;
  const line = buckets.map((bucket, i) => {
    const x = i * step;
    const y = height - (bucket.totalTokens / max) * height;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });
  return `M0,${height} L${line.join(" L")} L${width},${height} Z`;
});

const breakdownData = computed(() => {
  const buckets = timeseries.value;
  const groups = new Map<string, { tokens: number; requests: number; duration: number }>();
  // Breakdown is derived from logs for richer data; use logs as source
  for (const log of logs.value) {
    let key: string;
    if (breakdownTab.value === "provider") key = log.provider;
    else if (breakdownTab.value === "model") key = log.model;
    else key = log.accountLabel ?? "unknown";
    const existing = groups.get(key) ?? { tokens: 0, requests: 0, duration: 0 };
    existing.tokens += log.inputTokens + log.outputTokens;
    existing.requests += 1;
    existing.duration += log.latencyMs;
    groups.set(key, existing);
  }
  // If no logs, fall back to timeseries totals under a single bucket
  if (groups.size === 0 && buckets.length > 0) {
    const totalTokens = buckets.reduce((sum, b) => sum + b.totalTokens, 0);
    const totalRequests = buckets.reduce((sum, b) => sum + b.requestCount, 0);
    const avgDuration = buckets.length > 0
      ? buckets.reduce((sum, b) => sum + b.avgDurationMs, 0) / buckets.length
      : 0;
    groups.set("all", { tokens: totalTokens, requests: totalRequests, duration: avgDuration });
  }
  return Array.from(groups.entries())
    .map(([key, val]) => ({
      key,
      tokens: val.tokens,
      requests: val.requests,
      avgDuration: val.requests > 0 ? val.duration / val.requests : 0,
    }))
    .sort((a, b) => b.tokens - a.tokens);
});

function setDatePreset(preset: DatePreset) {
  datePreset.value = preset;
  if (preset === "custom") return;
  const end = new Date();
  let start: Date;
  switch (preset) {
    case "24h":
      start = new Date(end.getTime() - 24 * 60 * 60 * 1000);
      break;
    case "7d":
      start = new Date(end.getTime() - 7 * 24 * 60 * 60 * 1000);
      break;
    case "30d":
      start = new Date(end.getTime() - 30 * 24 * 60 * 60 * 1000);
      break;
  }
  range.value = { start: start.toISOString(), end: end.toISOString() };
}

function applyCustomRange() {
  if (!customStart.value || !customEnd.value) return;
  range.value = {
    start: new Date(customStart.value).toISOString(),
    end: new Date(customEnd.value).toISOString(),
  };
}

function intervalForPreset(preset: DatePreset): string {
  switch (preset) {
    case "24h": return "1h";
    case "7d": return "6h";
    case "30d": return "1d";
    case "custom": return "1h";
  }
}

async function loadAll() {
  loading.value = true;
  error.value = "";
  try {
    const [summaryData, timeseriesData] = await Promise.all([
      getUsageSummary(range.value.start, range.value.end),
      getUsageTimeseries(range.value.start, range.value.end, intervalForPreset(datePreset.value)),
    ]);
    summary.value = summaryData;
    timeseries.value = timeseriesData;
  } catch (requestError) {
    error.value = requestError instanceof Error ? requestError.message : "Usage data is unavailable.";
    summary.value = null;
    timeseries.value = [];
  } finally {
    loading.value = false;
  }
  logsPage.value = 0;
  await loadLogs();
}

async function loadLogs() {
  logsLoading.value = true;
  try {
    const data = await getUsageRequestLogs(LOGS_PER_PAGE + 1, logsPage.value * LOGS_PER_PAGE);
    logsHasMore.value = data.length > LOGS_PER_PAGE;
    logs.value = data.slice(0, LOGS_PER_PAGE);
  } catch {
    logs.value = [];
    logsHasMore.value = false;
  } finally {
    logsLoading.value = false;
  }
}

function nextPage() {
  if (!logsHasMore.value) return;
  logsPage.value += 1;
  void loadLogs();
}

function prevPage() {
  if (logsPage.value <= 0) return;
  logsPage.value -= 1;
  void loadLogs();
}

watch(range, () => {
  void loadAll();
});

watch(breakdownTab, () => {
  // Re-derive from existing logs; no fetch needed
});

onMounted(() => {
  void loadAll();
});
</script>

<template>
  <div class="usage-page animate-fade-in">
    <header class="usage-page__head">
      <div>
        <p class="text-caption uppercase tracking-[0.18em] text-muted">Analytics</p>
        <h1 class="settings__title">Usage Analytics</h1>
      </div>
      <div class="usage-page__head-actions">
        <div class="usage-page__presets">
          <button
            v-for="preset in (['24h', '7d', '30d', 'custom'] as DatePreset[])"
            :key="preset"
            type="button"
            class="usage-page__preset"
            :class="{ 'is-active': datePreset === preset }"
            @click="setDatePreset(preset)"
          >
            {{ preset === "24h" ? "24h" : preset === "7d" ? "7d" : preset === "30d" ? "30d" : "Custom" }}
          </button>
        </div>
        <div v-if="datePreset === 'custom'" class="usage-page__custom-range">
          <input v-model="customStart" type="datetime-local" class="input-luxury usage-page__datetime" />
          <span class="usage-page__range-sep">–</span>
          <input v-model="customEnd" type="datetime-local" class="input-luxury usage-page__datetime" />
          <button type="button" class="btn-secondary" @click="applyCustomRange">Apply</button>
        </div>
        <button type="button" class="btn-secondary" :disabled="loading" @click="loadAll">
          <RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': loading }" /> Refresh
        </button>
      </div>
    </header>

    <p class="usage-page__range-label">
      {{ startLabel }} — {{ endLabel }}
    </p>

    <div v-if="error" class="usage-page__alert" role="alert">
      <AlertCircle class="h-4 w-4" /> {{ error }}
    </div>

    <section v-if="loading && !summary" class="workbench-card usage-page__loading">
      <CircleDashed class="h-5 w-5 animate-spin" /> Loading usage data…
    </section>

    <template v-else-if="summary">
      <!-- Summary cards -->
      <section class="usage-page__summary-row">
        <div class="workbench-card usage-page__summary-card">
          <span class="usage-page__summary-label">Total Requests</span>
          <strong class="usage-page__summary-value">{{ summary.totalRequests.toLocaleString() }}</strong>
        </div>
        <div class="workbench-card usage-page__summary-card">
          <span class="usage-page__summary-label">Total Tokens</span>
          <strong class="usage-page__summary-value">{{ formatTokenCount(summary.totalTokens) }}</strong>
          <span class="usage-page__summary-sub">
            {{ formatTokenCount(summary.totalInputTokens) }} in / {{ formatTokenCount(summary.totalOutputTokens) }} out
          </span>
        </div>
        <div class="workbench-card usage-page__summary-card">
          <span class="usage-page__summary-label">Avg Latency</span>
          <strong class="usage-page__summary-value">{{ formatDuration(summary.avgDurationMs) }}</strong>
        </div>
        <div class="workbench-card usage-page__summary-card">
          <span class="usage-page__summary-label">Est. Cost</span>
          <strong class="usage-page__summary-value">{{ formatCost(summary.estimatedCostUsd) }}</strong>
        </div>
      </section>

      <!-- Time series chart -->
      <section class="workbench-card usage-page__chart-card">
        <div class="usage-page__chart-head">
          <BarChart3 class="h-4 w-4 text-muted" />
          <span class="text-caption uppercase tracking-[0.16em] text-muted">Token usage over time</span>
        </div>
        <div v-if="timeseries.length === 0" class="usage-page__chart-empty">
          No data points for this range.
        </div>
        <svg
          v-else
          class="usage-page__chart"
          viewBox="0 0 600 160"
          preserveAspectRatio="none"
          aria-label="Token usage time series"
        >
          <path :d="chartAreaPath" class="usage-page__chart-area" />
          <path :d="chartPath" class="usage-page__chart-line" />
        </svg>
      </section>

      <!-- Breakdown tabs -->
      <section class="workbench-card usage-page__breakdown-card">
        <div class="usage-page__breakdown-head">
          <span class="text-caption uppercase tracking-[0.16em] text-muted">Breakdown</span>
          <div class="usage-page__tabs">
            <button
              v-for="tab in (['provider', 'model', 'account'] as BreakdownTab[])"
              :key="tab"
              type="button"
              class="usage-page__tab"
              :class="{ 'is-active': breakdownTab === tab }"
              @click="breakdownTab = tab"
            >
              {{ tab === "provider" ? "By Provider" : tab === "model" ? "By Model" : "By Account" }}
            </button>
          </div>
        </div>
        <div v-if="breakdownData.length === 0" class="usage-page__breakdown-empty">
          No breakdown data available for this period.
        </div>
        <table v-else class="usage-page__breakdown-table">
          <thead>
            <tr>
              <th>{{ breakdownTab === "provider" ? "Provider" : breakdownTab === "model" ? "Model" : "Account" }}</th>
              <th class="usage-page__num">Requests</th>
              <th class="usage-page__num">Tokens</th>
              <th class="usage-page__num">Avg Latency</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="row in breakdownData" :key="row.key">
              <td>{{ row.key }}</td>
              <td class="usage-page__num">{{ row.requests.toLocaleString() }}</td>
              <td class="usage-page__num">{{ formatTokenCount(row.tokens) }}</td>
              <td class="usage-page__num">{{ formatDuration(row.avgDuration) }}</td>
            </tr>
          </tbody>
        </table>
      </section>

      <!-- Request logs -->
      <section class="workbench-card usage-page__logs-card">
        <div class="usage-page__logs-head">
          <span class="text-caption uppercase tracking-[0.16em] text-muted">Request logs</span>
          <span class="usage-page__logs-page">Page {{ logsPage + 1 }}</span>
        </div>
        <div v-if="logsLoading" class="usage-page__logs-loading">
          <CircleDashed class="h-4 w-4 animate-spin" /> Loading logs…
        </div>
        <template v-else-if="logs.length > 0">
          <div class="usage-page__logs-scroll">
            <table class="usage-page__logs-table">
              <thead>
                <tr>
                  <th>Time</th>
                  <th>Provider</th>
                  <th>Model</th>
                  <th class="usage-page__num">In</th>
                  <th class="usage-page__num">Out</th>
                  <th class="usage-page__num">Latency</th>
                  <th>Status</th>
                  <th>Source</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="log in logs" :key="log.id">
                  <td class="usage-page__log-time">{{ new Date(log.timestamp).toLocaleTimeString() }}</td>
                  <td>{{ log.provider }}</td>
                  <td class="usage-page__log-model">{{ log.model }}</td>
                  <td class="usage-page__num">{{ formatTokenCount(log.inputTokens) }}</td>
                  <td class="usage-page__num">{{ formatTokenCount(log.outputTokens) }}</td>
                  <td class="usage-page__num">{{ formatDuration(log.latencyMs) }}</td>
                  <td>
                    <span
                      v-if="log.statusCode !== null"
                      class="usage-page__status"
                      :class="log.statusCode < 400 ? 'is-ok' : 'is-error'"
                    >
                      {{ log.statusCode }}
                    </span>
                    <span v-else class="text-muted">—</span>
                  </td>
                  <td>
                    <span class="chip" :class="log.source === 'agent_turn' ? 'chip-success' : 'chip-muted'">
                      {{ log.source === "agent_turn" ? "Agent" : "Proxy" }}
                    </span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div class="usage-page__logs-pagination">
            <button type="button" class="btn-ghost" :disabled="logsPage === 0" @click="prevPage">
              <ChevronLeft class="h-3.5 w-3.5" /> Prev
            </button>
            <button type="button" class="btn-ghost" :disabled="!logsHasMore" @click="nextPage">
              Next <ChevronRight class="h-3.5 w-3.5" />
            </button>
          </div>
        </template>
        <div v-else class="usage-page__logs-empty">
          No request logs for this period.
        </div>
      </section>
    </template>

    <section v-else class="workbench-card usage-page__loading">
      <AlertCircle class="h-5 w-5" /> Usage data is unavailable. Check the built-in proxy runtime.
    </section>
  </div>
</template>

<style scoped>
.usage-page { display: flex; flex-direction: column; gap: 1rem; max-width: 1180px; }
.usage-page__head { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; flex-wrap: wrap; }
.usage-page__head-actions { display: flex; align-items: center; gap: 0.55rem; flex-wrap: wrap; }
.usage-page__presets { display: flex; gap: 0.25rem; }
.usage-page__preset {
  padding: 0.35rem 0.65rem;
  border-radius: 0.5rem;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--muted-foreground);
  font-size: 0.7rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}
.usage-page__preset:hover { background: var(--surface-2); color: var(--foreground); }
.usage-page__preset.is-active { background: var(--surface-3); color: var(--foreground); border-color: var(--border-strong); }
.usage-page__custom-range { display: flex; align-items: center; gap: 0.35rem; }
.usage-page__datetime { font-size: 0.7rem; padding: 0.3rem 0.45rem; width: auto; }
.usage-page__range-sep { color: var(--muted-foreground); font-size: 0.7rem; }
.usage-page__range-label { color: var(--muted-foreground); font-size: 0.72rem; margin: -0.5rem 0 0; }
.usage-page__alert {
  display: flex; align-items: center; gap: 0.45rem;
  padding: 0.7rem 0.8rem; border: 1px solid var(--border); border-radius: 0.7rem;
  font-size: 0.76rem; color: var(--accent-red);
}
.usage-page__loading { display: flex; align-items: center; gap: 0.55rem; min-height: 8rem; padding: 1rem; color: var(--muted-foreground); font-size: 0.8rem; }
.usage-page__summary-row { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 0.75rem; }
.usage-page__summary-card { padding: 0.9rem 1rem; display: flex; flex-direction: column; gap: 0.25rem; }
.usage-page__summary-label { color: var(--muted-foreground); font-size: 0.68rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em; }
.usage-page__summary-value { font-size: 1.3rem; font-weight: 600; color: var(--foreground); }
.usage-page__summary-sub { color: var(--muted-foreground); font-size: 0.66rem; }
.usage-page__chart-card { padding: 1rem; }
.usage-page__chart-head { display: flex; align-items: center; gap: 0.45rem; margin-bottom: 0.75rem; }
.usage-page__chart-empty { padding: 2rem 0; text-align: center; color: var(--muted-foreground); font-size: 0.76rem; }
.usage-page__chart { width: 100%; height: 160px; display: block; }
.usage-page__chart-area { fill: var(--accent-green); opacity: 0.08; }
.usage-page__chart-line { fill: none; stroke: var(--accent-green); stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
.usage-page__breakdown-card { padding: 1rem; }
.usage-page__breakdown-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.75rem; }
.usage-page__tabs { display: flex; gap: 0.25rem; }
.usage-page__tab {
  padding: 0.3rem 0.6rem;
  border-radius: 0.45rem;
  border: 1px solid transparent;
  background: transparent;
  color: var(--muted-foreground);
  font-size: 0.68rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}
.usage-page__tab:hover { background: var(--surface-2); }
.usage-page__tab.is-active { background: var(--surface-3); color: var(--foreground); border-color: var(--border); }
.usage-page__breakdown-empty { padding: 1.5rem 0; text-align: center; color: var(--muted-foreground); font-size: 0.76rem; }
.usage-page__breakdown-table { width: 100%; border-collapse: collapse; font-size: 0.76rem; }
.usage-page__breakdown-table th {
  text-align: left; padding: 0.5rem 0.6rem; border-bottom: 1px solid var(--border);
  color: var(--muted-foreground); font-size: 0.66rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em;
}
.usage-page__breakdown-table td { padding: 0.55rem 0.6rem; border-bottom: 1px solid var(--border); color: var(--foreground); }
.usage-page__breakdown-table tr:last-child td { border-bottom: none; }
.usage-page__num { text-align: right; font-variant-numeric: tabular-nums; }
.usage-page__logs-card { padding: 1rem; }
.usage-page__logs-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 0.75rem; }
.usage-page__logs-page { color: var(--muted-foreground); font-size: 0.68rem; }
.usage-page__logs-loading { display: flex; align-items: center; gap: 0.45rem; padding: 1.5rem 0; justify-content: center; color: var(--muted-foreground); font-size: 0.76rem; }
.usage-page__logs-scroll { overflow-x: auto; }
.usage-page__logs-table { width: 100%; border-collapse: collapse; font-size: 0.72rem; }
.usage-page__logs-table th {
  text-align: left; padding: 0.45rem 0.5rem; border-bottom: 1px solid var(--border);
  color: var(--muted-foreground); font-size: 0.64rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em;
  white-space: nowrap;
}
.usage-page__logs-table td { padding: 0.45rem 0.5rem; border-bottom: 1px solid var(--border); color: var(--foreground); white-space: nowrap; }
.usage-page__logs-table tr:last-child td { border-bottom: none; }
.usage-page__log-time { color: var(--muted-foreground); font-variant-numeric: tabular-nums; }
.usage-page__log-model { max-width: 200px; overflow: hidden; text-overflow: ellipsis; }
.usage-page__status { font-size: 0.66rem; font-weight: 600; padding: 0.15rem 0.4rem; border-radius: 0.35rem; }
.usage-page__status.is-ok { color: var(--accent-green); background: color-mix(in srgb, var(--accent-green) 12%, transparent); }
.usage-page__status.is-error { color: var(--accent-red); background: color-mix(in srgb, var(--accent-red) 12%, transparent); }
.usage-page__logs-pagination { display: flex; align-items: center; justify-content: space-between; margin-top: 0.75rem; }
.usage-page__logs-empty { padding: 2rem 0; text-align: center; color: var(--muted-foreground); font-size: 0.76rem; }
@media (max-width: 820px) {
  .usage-page__head { flex-direction: column; }
  .usage-page__summary-row { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}
@media (max-width: 520px) {
  .usage-page__summary-row { grid-template-columns: 1fr; }
}
</style>
