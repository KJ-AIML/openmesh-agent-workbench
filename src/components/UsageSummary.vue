<script setup lang="ts">
import { RefreshCw } from "lucide-vue-next";
import { computed } from "vue";
import type { UsageSummary } from "../types";
import { formatTokens, formatRequests } from "../lib/format";

const props = defineProps<{
  summary: UsageSummary;
  loading?: boolean;
}>();

const emit = defineEmits<{ (e: "refresh"): void }>();

const tokensLabel = computed(() => formatTokens(props.summary.totalTokens));
const requestsLabel = computed(() =>
  formatRequests(props.summary.totalRequests)
);
const promptLabel = computed(() => formatTokens(props.summary.promptTokens));
const completionLabel = computed(() =>
  formatTokens(props.summary.completionTokens)
);
const req24hLabel = computed(() =>
  formatRequests(props.summary.requests24h)
);
const avgLabel = computed(() => formatTokens(props.summary.avgTokens));
const avgInLabel = computed(() => formatTokens(props.summary.avgTokensIn));
const avgOutLabel = computed(() => formatTokens(props.summary.avgTokensOut));
</script>

<template>
  <section>
    <div class="flex items-start justify-between gap-4">
      <div>
        <div
          class="text-[11px] font-medium uppercase tracking-wider"
          style="color: var(--muted-foreground)"
        >
          USAGE controller
        </div>
        <h1
          class="mt-1 text-3xl font-semibold tracking-tight tabular-nums"
          style="color: var(--foreground)"
        >
          {{ tokensLabel }}<span class="ml-2 text-xl opacity-80">tokens</span>
        </h1>
        <p
          class="mt-1 text-sm tabular-nums"
          style="color: var(--muted-foreground)"
        >
          {{ requestsLabel }} requests ·
          {{ summary.sessions }} sessions ·
          {{ summary.users }} users
        </p>
      </div>
    </div>

    <div class="mt-4 grid grid-cols-2 gap-3 lg:grid-cols-4">
      <div class="rounded-lg border p-4" style="border-color: var(--border); background: var(--card)">
        <div
          class="text-[11px] font-medium uppercase tracking-wider"
          style="color: var(--muted-foreground)"
        >
          Prompt
        </div>
        <div
          class="mt-2 text-xl font-semibold tabular-nums"
          style="color: var(--foreground)"
        >
          {{ promptLabel }}
        </div>
        <div class="mt-1 text-xs tabular-nums" style="color: var(--muted-foreground)">
          input tokens
        </div>
      </div>

      <div class="rounded-lg border p-4" style="border-color: var(--border); background: var(--card)">
        <div
          class="text-[11px] font-medium uppercase tracking-wider"
          style="color: var(--muted-foreground)"
        >
          Completion
        </div>
        <div
          class="mt-2 text-xl font-semibold tabular-nums"
          style="color: var(--foreground)"
        >
          {{ completionLabel }}
        </div>
        <div class="mt-1 text-xs tabular-nums" style="color: var(--muted-foreground)">
          output tokens
        </div>
      </div>

      <div class="rounded-lg border p-4" style="border-color: var(--border); background: var(--card)">
        <div
          class="text-[11px] font-medium uppercase tracking-wider"
          style="color: var(--muted-foreground)"
        >
          24H REQ
        </div>
        <div
          class="mt-2 text-xl font-semibold tabular-nums"
          style="color: var(--foreground)"
        >
          {{ req24hLabel }}
        </div>
        <div class="mt-1 text-xs tabular-nums" style="color: var(--muted-foreground)">
          {{ summary.requestsLastHour }} last hour
        </div>
      </div>

      <div class="rounded-lg border p-4" style="border-color: var(--border); background: var(--card)">
        <div
          class="text-[11px] font-medium uppercase tracking-wider"
          style="color: var(--muted-foreground)"
        >
          Avg Tokens
        </div>
        <div
          class="mt-2 text-xl font-semibold tabular-nums"
          style="color: var(--foreground)"
        >
          {{ avgLabel }}
        </div>
        <div class="mt-1 text-xs tabular-nums" style="color: var(--muted-foreground)">
          {{ avgInLabel }} in · {{ avgOutLabel }} out
        </div>
      </div>

      <div class="col-span-2 lg:col-span-4 flex justify-end">
        <button
          type="button"
          aria-label="Refresh"
          class="flex items-center gap-2 rounded-md border px-3 py-1.5 text-xs transition-colors hover:text-[var(--foreground)] hover:bg-[var(--sidebar-accent)]"
          style="
            border-color: var(--border);
            background: var(--card);
            color: var(--muted-foreground);
          "
          @click="emit('refresh')"
        >
          <RefreshCw class="h-3.5 w-3.5" :class="loading ? 'animate-spin' : ''" />
          Refresh
        </button>
      </div>
    </div>
  </section>
</template>
