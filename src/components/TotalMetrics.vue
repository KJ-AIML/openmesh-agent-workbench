<script setup lang="ts">
import { computed } from "vue";
import { formatTokens, formatRequests } from "../lib/format";

const props = defineProps<{
  totalTokens: number;
  totalRequests: number;
  peakBucket: number;
}>();

const tokensLabel = computed(() => formatTokens(props.totalTokens));
const requestsLabel = computed(() => formatRequests(props.totalRequests));
const peakLabel = computed(() => formatTokens(props.peakBucket));
</script>

<template>
  <section class="grid grid-cols-1 gap-3 sm:grid-cols-3">
    <div
      class="rounded-lg border p-4"
      style="border-color: var(--border); background: var(--card)"
    >
      <div
        class="text-[11px] font-medium uppercase tracking-wider"
        style="color: var(--muted-foreground)"
      >
        Total Tokens
      </div>
      <div
        class="mt-2 text-2xl font-semibold tabular-nums"
        style="color: var(--foreground)"
      >
        {{ tokensLabel }}
      </div>
    </div>

    <div
      class="rounded-lg border p-4"
      style="border-color: var(--border); background: var(--card)"
    >
      <div
        class="text-[11px] font-medium uppercase tracking-wider"
        style="color: var(--muted-foreground)"
      >
        Total Requests
      </div>
      <div
        class="mt-2 text-2xl font-semibold tabular-nums"
        style="color: var(--foreground)"
      >
        {{ requestsLabel }}
      </div>
    </div>

    <div
      class="rounded-lg border p-4"
      style="border-color: var(--border); background: var(--card)"
    >
      <div
        class="text-[11px] font-medium uppercase tracking-wider"
        style="color: var(--muted-foreground)"
      >
        Peak Bucket
      </div>
      <div
        class="mt-2 text-2xl font-semibold tabular-nums"
        style="color: var(--foreground)"
      >
        {{ peakLabel }}
      </div>
    </div>
  </section>
</template>
