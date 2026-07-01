<script setup lang="ts">
import { computed, ref } from "vue";
import { ArrowDown, ArrowUp, ArrowUpDown } from "lucide-vue-next";
import type { ModelMetrics } from "../types";
import { colorForModel } from "../types";
import {
  formatTokens,
  formatRequests,
  formatLatency,
  formatSpeed,
} from "../lib/format";

const props = defineProps<{
  models: ModelMetrics[];
}>();

type SortKey = "requests" | "tokens" | "latency" | "ttft" | "speed";

const sortKey = ref<SortKey>("tokens");
const asc = ref(false);

function toggleSort(k: SortKey) {
  if (k === sortKey.value) asc.value = !asc.value;
  else {
    sortKey.value = k;
    asc.value = false;
  }
}

const sorted = computed(() => {
  const arr = [...props.models];
  const k = sortKey.value;
  arr.sort((a, b) => {
    let av: number, bv: number;
    switch (k) {
      case "requests":
        av = a.requests;
        bv = b.requests;
        break;
      case "tokens":
        av = a.tokens;
        bv = b.tokens;
        break;
      case "latency":
        av = a.latency;
        bv = b.latency;
        break;
      case "ttft":
        av = a.ttft;
        bv = b.ttft;
        break;
      case "speed":
        av = a.prefill + a.gen;
        bv = b.prefill + b.gen;
        break;
    }
    return asc.value ? av - bv : bv - av;
  });
  return arr;
});

const maxReq = computed(() =>
  Math.max(1, ...props.models.map((m) => m.requests))
);
const maxTok = computed(() =>
  Math.max(1, ...props.models.map((m) => m.tokens))
);
const maxLat = computed(() =>
  Math.max(1, ...props.models.map((m) => m.latency))
);
const maxTtft = computed(() =>
  Math.max(1, ...props.models.map((m) => m.ttft))
);
const maxSpeed = computed(() =>
  Math.max(1, ...props.models.map((m) => m.prefill + m.gen))
);

function pct(value: number, max: number): number {
  if (max <= 0) return 0;
  return Math.max(2, Math.min(100, (value / max) * 100));
}

function sortIcon(k: SortKey) {
  if (k !== sortKey.value) return "none";
  return asc.value ? "up" : "down";
}

const columns: { key: SortKey; label: string }[] = [
  { key: "requests", label: "Requests" },
  { key: "tokens", label: "Tokens" },
  { key: "latency", label: "Latency" },
  { key: "ttft", label: "TTFT" },
  { key: "speed", label: "Speed" },
];
</script>

<template>
  <section
    class="rounded-lg border"
    style="border-color: var(--border); background: var(--card)"
  >
    <div
      class="flex items-center justify-between border-b px-4 py-3"
      style="border-color: var(--border)"
    >
      <div
        class="text-[11px] font-medium uppercase tracking-wider"
        style="color: var(--muted-foreground)"
      >
        Model Performance
      </div>
      <div class="text-xs" style="color: var(--muted-foreground)">
        {{ models.length }} model{{ models.length === 1 ? "" : "s" }}
      </div>
    </div>

    <div v-if="models.length === 0" class="px-4 py-16 text-center">
      <div class="text-sm" style="color: var(--muted-foreground)">No models</div>
      <div class="mt-1 text-xs opacity-60" style="color: var(--muted-foreground)">
        Model performance metrics will appear here once requests have been made.
      </div>
    </div>

    <div v-else class="overflow-x-auto">
      <table class="w-full min-w-[760px] text-sm">
        <thead>
          <tr
            class="text-[11px] uppercase tracking-wider"
            style="color: var(--muted-foreground)"
          >
            <th class="px-4 py-2.5 text-left font-medium">Model</th>
            <th
              v-for="col in columns"
              :key="col.key"
              class="px-4 py-2.5 text-left font-medium"
            >
              <button
                type="button"
                class="flex items-center gap-1 hover:text-[var(--foreground)] transition-colors"
                @click="toggleSort(col.key)"
              >
                {{ col.label }}
                <ArrowUpDown
                  v-if="sortIcon(col.key) === 'none'"
                  class="h-3 w-3 opacity-40"
                />
                <ArrowUp
                  v-else-if="sortIcon(col.key) === 'up'"
                  class="h-3 w-3"
                  style="color: var(--foreground)"
                />
                <ArrowDown
                  v-else
                  class="h-3 w-3"
                  style="color: var(--foreground)"
                />
              </button>
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(m, idx) in sorted"
            :key="m.model"
            class="border-t transition-colors hover:bg-[var(--sidebar-accent)]"
            style="border-color: rgba(255, 255, 255, 0.04)"
          >
            <td class="px-4 py-3">
              <div class="flex items-center gap-2">
                <span
                  class="h-2.5 w-2.5 rounded-sm"
                  :style="{ background: colorForModel(m.model, idx) }"
                />
                <span class="font-medium" style="color: var(--foreground)">
                  {{ m.model }}
                </span>
              </div>
            </td>

            <!-- Requests -->
            <td class="px-4 py-3">
              <div class="flex items-center gap-2">
                <div
                  class="relative h-1.5 w-20 overflow-hidden rounded-full"
                  style="background: var(--muted)"
                >
                  <div
                    class="absolute inset-y-0 left-0 rounded-full"
                    :style="{
                      width: pct(m.requests, maxReq) + '%',
                      background: '#50C878',
                    }"
                  />
                </div>
                <span
                  class="text-xs tabular-nums"
                  style="color: var(--foreground)"
                >
                  {{ formatRequests(m.requests) }}
                </span>
              </div>
            </td>

            <!-- Tokens -->
            <td class="px-4 py-3">
              <div class="flex items-center gap-2">
                <div
                  class="relative h-1.5 w-20 overflow-hidden rounded-full"
                  style="background: var(--muted)"
                >
                  <div
                    class="absolute inset-y-0 left-0 rounded-full"
                    :style="{
                      width: pct(m.tokens, maxTok) + '%',
                      background: '#50C878',
                    }"
                  />
                </div>
                <span
                  class="text-xs tabular-nums"
                  style="color: var(--foreground)"
                >
                  {{ formatTokens(m.tokens) }}
                </span>
              </div>
            </td>

            <!-- Latency -->
            <td class="px-4 py-3">
              <div class="flex items-center gap-2">
                <div
                  class="relative h-1.5 w-20 overflow-hidden rounded-full"
                  style="background: var(--muted)"
                >
                  <div
                    class="absolute inset-y-0 left-0 rounded-full"
                    :style="{
                      width: pct(m.latency, maxLat) + '%',
                      background: '#FF6B6B',
                    }"
                  />
                </div>
                <span
                  class="text-xs tabular-nums"
                  style="color: var(--foreground)"
                >
                  {{ formatLatency(m.latency) }}
                </span>
              </div>
            </td>

            <!-- TTFT -->
            <td class="px-4 py-3">
              <div class="flex items-center gap-2">
                <div
                  class="relative h-1.5 w-20 overflow-hidden rounded-full"
                  style="background: var(--muted)"
                >
                  <div
                    class="absolute inset-y-0 left-0 rounded-full"
                    :style="{
                      width: pct(m.ttft, maxTtft) + '%',
                      background: '#FF6B6B',
                    }"
                  />
                </div>
                <span
                  class="text-xs tabular-nums"
                  style="color: var(--foreground)"
                >
                  {{ formatLatency(m.ttft) }}
                </span>
              </div>
            </td>

            <!-- Speed -->
            <td class="px-4 py-3">
              <div class="flex items-center gap-2">
                <div
                  class="relative h-1.5 w-20 overflow-hidden rounded-full"
                  style="background: var(--muted)"
                >
                  <div
                    class="absolute inset-y-0 left-0 rounded-full"
                    :style="{
                      width: pct(m.prefill + m.gen, maxSpeed) + '%',
                      background: '#50C878',
                    }"
                  />
                </div>
                <span
                  class="text-xs tabular-nums"
                  style="color: var(--foreground)"
                >
                  {{ formatSpeed(m.prefill, m.gen) }}
                </span>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
