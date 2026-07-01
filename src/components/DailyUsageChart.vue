<script setup lang="ts">
import { computed, ref, shallowRef, watch, onMounted, onUnmounted } from "vue";
import * as echarts from "echarts/core";
import { BarChart } from "echarts/charts";
import {
  GridComponent,
  TooltipComponent,
  LegendComponent,
} from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import type { DailyBucket } from "../types";
import { colorForModel } from "../types";
import { formatNumber } from "../lib/format";

echarts.use([
  BarChart,
  GridComponent,
  TooltipComponent,
  LegendComponent,
  CanvasRenderer,
]);

const props = defineProps<{
  daily: DailyBucket[];
}>();

const chartEl = ref<HTMLDivElement | null>(null);
const chartInstance = shallowRef<echarts.ECharts | null>(null);

const hiddenModels = ref<Set<string>>(new Set());

const modelsInData = computed(() => {
  const s = new Set<string>();
  for (const d of props.daily) {
    for (const k of Object.keys(d.values)) s.add(k);
  }
  return Array.from(s);
});

const option = computed(() => {
  const dates = props.daily.map((d) => d.label);
  const models = modelsInData.value.filter((m) => !hiddenModels.value.has(m));

  const series = models.map((m, idx) => ({
    name: m,
    type: "bar" as const,
    stack: "total",
    barWidth: "60%",
    itemStyle: { color: colorForModel(m, idx) },
    data: props.daily.map((d) => d.values[m] ?? 0),
  }));

  return {
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "shadow" },
      backgroundColor: "rgba(26, 26, 26, 0.95)",
      borderColor: "rgba(255, 255, 255, 0.08)",
      textStyle: { color: "#fafafa", fontSize: 12 },
      formatter: (params: any[]) => {
        if (!params.length) return "";
        const p0 = params[0];
        const bucket = props.daily[p0.dataIndex];
        if (!bucket) return "";
        const total = bucket.total;
        const lines = params
          .map((p) => {
            const pct = total > 0 ? ((p.value / total) * 100).toFixed(1) : "0.0";
            return `<div style="display:flex;align-items:center;gap:6px;margin-top:2px;">
              <span style="display:inline-block;width:8px;height:8px;border-radius:2px;background:${p.color}"></span>
              <span>${p.seriesName}</span>
              <span style="margin-left:auto;color:#888">${formatNumber(
                p.value
              )} · ${pct}%</span>
            </div>`;
          })
          .join("");
        return `<div style="font-weight:600">${bucket.label} · ${formatNumber(
          total
        )} REQ</div>
        <div style="color:#888;font-size:11px">${formatNumber(
          bucket.totalTokens
        )} tokens</div>
        <div style="margin-top:4px">${lines}</div>`;
      },
    },
    grid: { top: 12, right: 16, bottom: 8, left: 48, containLabel: false },
    xAxis: {
      type: "category",
      data: dates,
      axisLine: { lineStyle: { color: "rgba(255,255,255,0.1)" } },
      axisTick: { show: false },
      axisLabel: { color: "#888", fontSize: 11, interval: 3 },
    },
    yAxis: {
      type: "value",
      axisLine: { show: false },
      axisTick: { show: false },
      splitLine: { lineStyle: { color: "rgba(255,255,255,0.05)" } },
      axisLabel: { color: "#888", fontSize: 11 },
    },
    series,
  };
});

function renderChart() {
  if (!chartEl.value) return;
  if (!chartInstance.value) {
    chartInstance.value = echarts.init(chartEl.value, "dark");
  }
  chartInstance.value.setOption(option.value, true);
}

function toggleModel(m: string) {
  const next = new Set(hiddenModels.value);
  if (next.has(m)) next.delete(m);
  else next.add(m);
  hiddenModels.value = next;
}

function handleResize() {
  chartInstance.value?.resize();
}

onMounted(() => {
  renderChart();
  window.addEventListener("resize", handleResize);
});

onUnmounted(() => {
  window.removeEventListener("resize", handleResize);
  chartInstance.value?.dispose();
});

watch(
  () => [props.daily, hiddenModels.value],
  () => renderChart(),
  { deep: true }
);
</script>

<template>
  <section
    class="rounded-lg border p-4"
    style="border-color: var(--border); background: var(--card)"
  >
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div>
        <div
          class="text-[11px] font-medium uppercase tracking-wider"
          style="color: var(--muted-foreground)"
        >
          Daily Usage
        </div>
        <div class="mt-0.5 text-sm" style="color: var(--foreground)">
          Requests per day, stacked by model
        </div>
      </div>
    </div>

    <div v-if="daily.length === 0" class="mt-4 h-64 flex items-center justify-center">
      <div class="text-center">
        <div class="text-sm" style="color: var(--muted-foreground)">No usage data</div>
        <div class="mt-1 text-xs opacity-60" style="color: var(--muted-foreground)">
          Daily request volumes will appear here once activity is recorded.
        </div>
      </div>
    </div>

    <div v-else ref="chartEl" class="mt-4 h-64 w-full"></div>

    <!-- Legend -->
    <div v-if="modelsInData.length > 0" class="mt-3 flex flex-wrap gap-x-4 gap-y-2">
      <button
        v-for="(m, idx) in modelsInData"
        :key="m"
        type="button"
        class="flex items-center gap-1.5 text-xs transition-colors hover:text-[var(--foreground)]"
        :style="{
          color: hiddenModels.has(m) ? 'var(--muted-foreground)' : 'var(--muted-foreground)',
          opacity: hiddenModels.has(m) ? 0.4 : 1,
        }"
        @click="toggleModel(m)"
      >
        <span
          class="h-2.5 w-2.5 rounded-sm"
          :style="{ background: colorForModel(m, idx) }"
        />
        {{ m }}
      </button>
    </div>
  </section>
</template>
