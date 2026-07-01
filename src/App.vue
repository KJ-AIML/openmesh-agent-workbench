<script setup lang="ts">
import { onMounted, ref } from "vue";
import Sidebar from "./components/Sidebar.vue";
import UsageSummary from "./components/UsageSummary.vue";
import DailyUsageChart from "./components/DailyUsageChart.vue";
import TotalMetrics from "./components/TotalMetrics.vue";
import ModelPerformanceTable from "./components/ModelPerformanceTable.vue";
import { EMPTY_USAGE, type DashboardData } from "./types";

const TOP_NAV = ["SOURCE", "Provider", "Pi sessions"];

const data = ref<DashboardData>(EMPTY_USAGE);
const loading = ref(false);
const error = ref<string | null>(null);
const activeTab = ref("SOURCE");

const projects = ref<string[]>([]);

async function fetchData() {
  loading.value = true;
  error.value = null;
  try {
    const res = await fetch("/api/usage");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const json = (await res.json()) as Partial<DashboardData>;
    // Merge defensively — never invent fields.
    data.value = {
      summary: { ...EMPTY_USAGE.summary, ...(json.summary ?? {}) },
      daily: Array.isArray(json.daily) ? json.daily : [],
      totals: { ...EMPTY_USAGE.totals, ...(json.totals ?? {}) },
      models: Array.isArray(json.models) ? json.models : [],
    };
    // Side-load projects list if backend provides one
    try {
      const pRes = await fetch("/api/projects");
      if (pRes.ok) {
        const pj = await pRes.json();
        if (Array.isArray(pj.items)) projects.value = pj.items as string[];
      }
    } catch {
      // No projects endpoint — keep empty list
    }
  } catch (e: any) {
    // No backend available — stay with EMPTY_USAGE (no mock data).
    error.value = e?.message ?? "Failed to load usage data";
    data.value = EMPTY_USAGE;
    projects.value = [];
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  fetchData();
});
</script>

<template>
  <div
    class="flex min-h-screen w-full"
    style="background: var(--background); color: var(--foreground)"
  >
    <Sidebar :projects="projects" />

    <!-- Main content -->
    <div class="flex flex-1 flex-col min-w-0">
      <!-- Top mini nav -->
      <header
        class="sticky top-0 z-10 flex h-14 items-center gap-1 border-b px-4 backdrop-blur"
        style="
          border-color: var(--border);
          background: color-mix(in srgb, var(--background) 80%, transparent);
        "
      >
        <nav class="flex items-center gap-1 text-sm">
          <template v-for="(t, idx) in TOP_NAV" :key="t">
            <button
              type="button"
              class="rounded-md px-2 py-1 transition-colors"
              :style="{
                color:
                  activeTab === t
                    ? 'var(--foreground)'
                    : 'var(--muted-foreground)',
              }"
              @click="activeTab = t"
              @mouseenter="($event.target as HTMLElement).style.color = 'var(--foreground)'"
              @mouseleave="($event.target as HTMLElement).style.color = activeTab === t ? 'var(--foreground)' : 'var(--muted-foreground)'"
            >
              {{ t }}
            </button>
            <span
              v-if="idx < TOP_NAV.length - 1"
              class="px-1 opacity-40"
              style="color: var(--muted-foreground)"
              >/</span
            >
          </template>
        </nav>
      </header>

      <main class="flex-1 space-y-6 p-4 md:p-6 lg:p-8">
        <UsageSummary
          :summary="data.summary"
          :loading="loading"
          @refresh="fetchData"
        />
        <DailyUsageChart :daily="data.daily" />
        <TotalMetrics
          :total-tokens="data.totals.totalTokens"
          :total-requests="data.totals.totalRequests"
          :peak-bucket="data.totals.peakBucket"
        />
        <ModelPerformanceTable :models="data.models" />
      </main>
    </div>
  </div>
</template>
