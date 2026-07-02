<script setup lang="ts">
import { onMounted, ref } from "vue";
import UsageSummary from "../components/UsageSummary.vue";
import DailyUsageChart from "../components/DailyUsageChart.vue";
import TotalMetrics from "../components/TotalMetrics.vue";
import ModelPerformanceTable from "../components/ModelPerformanceTable.vue";
import { EMPTY_USAGE, type DashboardData } from "../types";

const data = ref<DashboardData>(EMPTY_USAGE);
const loading = ref(false);
const error = ref<string | null>(null);

async function fetchData() {
  loading.value = true;
  error.value = null;
  try {
    const res = await fetch("/api/usage");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const json = (await res.json()) as Partial<DashboardData>;
    data.value = {
      summary: { ...EMPTY_USAGE.summary, ...(json.summary ?? {}) },
      daily: Array.isArray(json.daily) ? json.daily : [],
      totals: { ...EMPTY_USAGE.totals, ...(json.totals ?? {}) },
      models: Array.isArray(json.models) ? json.models : [],
    };
  } catch (e: any) {
    error.value = e?.message ?? "Failed to load usage data";
    data.value = EMPTY_USAGE;
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  fetchData();
});
</script>

<template>
  <div class="space-y-6">
    <UsageSummary :summary="data.summary" :loading="loading" @refresh="fetchData" />
    <DailyUsageChart :daily="data.daily" />
    <TotalMetrics
      :total-tokens="data.totals.totalTokens"
      :total-requests="data.totals.totalRequests"
      :peak-bucket="data.totals.peakBucket"
    />
    <ModelPerformanceTable :models="data.models" />
  </div>
</template>
