<script setup lang="ts">
import { computed } from "vue";
import { AlertCircle } from "lucide-vue-next";
import { useQuotaPolling } from "../lib/quotaClient";
import { formatTokenCount } from "../lib/usageClient";

const { quotas, loading, error } = useQuotaPolling(60000);

const hasData = computed(() => quotas.value.length > 0);

type BarSegment = {
  provider: string;
  ratio: number;
  color: string;
  label: string;
  detail: string;
};

const segments = computed<BarSegment[]>(() => {
  return quotas.value.map((quota) => {
    let ratio = 1;
    if (quota.tokensRemaining !== null && quota.tokensLimit !== null && quota.tokensLimit > 0) {
      ratio = quota.tokensRemaining / quota.tokensLimit;
    } else if (quota.requestsRemaining !== null && quota.requestsLimit !== null && quota.requestsLimit > 0) {
      ratio = quota.requestsRemaining / quota.requestsLimit;
    } else {
      ratio = 1;
    }
    ratio = Math.max(0, Math.min(1, ratio));
    const color = ratio > 0.5 ? "var(--accent-green)" : ratio > 0.2 ? "var(--accent-yellow, var(--muted-foreground))" : "var(--accent-red)";
    const remaining = quota.tokensRemaining !== null
      ? `${formatTokenCount(quota.tokensRemaining)} tokens`
      : quota.requestsRemaining !== null
        ? `${quota.requestsRemaining} requests`
        : "—";
    const limit = quota.tokensLimit !== null
      ? `${formatTokenCount(quota.tokensLimit)} tokens`
      : quota.requestsLimit !== null
        ? `${quota.requestsLimit} requests`
        : "—";
    return {
      provider: quota.provider,
      ratio,
      color,
      label: quota.provider,
      detail: `${remaining} / ${limit}${quota.resetsAt ? ` · resets ${new Date(quota.resetsAt).toLocaleString()}` : ""}`,
    };
  });
});
</script>

<template>
  <div class="quota-bar" :class="{ 'quota-bar--empty': !hasData && !loading }">
    <div v-if="loading && !hasData" class="quota-bar__loading">
      Loading quota…
    </div>
    <div v-else-if="error && !hasData" class="quota-bar__unavailable">
      <AlertCircle class="h-3 w-3" />
      <span>Quota data unavailable</span>
    </div>
    <div v-else-if="!hasData" class="quota-bar__unavailable">
      <span>Quota data unavailable</span>
    </div>
    <template v-else>
      <div class="quota-bar__track">
        <div
          v-for="segment in segments"
          :key="segment.provider"
          class="quota-bar__segment"
          :style="{ flex: '1 1 0', background: segment.color, opacity: 0.25 + segment.ratio * 0.75 }"
          :title="`${segment.label}: ${segment.detail}`"
        >
          <span class="quota-bar__segment-label">{{ segment.label }}</span>
        </div>
      </div>
      <div class="quota-bar__legend">
        <span
          v-for="segment in segments"
          :key="segment.provider"
          class="quota-bar__legend-item"
          :title="segment.detail"
        >
          <span class="quota-bar__legend-dot" :style="{ background: segment.color }" />
          {{ segment.label }}
        </span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.quota-bar {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
  padding: 0.4rem 0.75rem;
  border-bottom: 1px solid var(--border);
  background: var(--sidebar);
}

.quota-bar--empty {
  padding: 0.3rem 0.75rem;
}

.quota-bar__loading,
.quota-bar__unavailable {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  color: var(--muted-foreground);
  font-size: 0.62rem;
  opacity: 0.7;
}

.quota-bar__unavailable svg {
  flex-shrink: 0;
}

.quota-bar__track {
  display: flex;
  gap: 2px;
  height: 6px;
  border-radius: 3px;
  overflow: hidden;
}

.quota-bar__segment {
  position: relative;
  min-width: 0;
  border-radius: 2px;
  transition: opacity 0.2s ease;
}

.quota-bar__segment:hover {
  opacity: 1 !important;
}

.quota-bar__segment-label {
  display: none;
}

.quota-bar__legend {
  display: flex;
  gap: 0.6rem;
  flex-wrap: wrap;
}

.quota-bar__legend-item {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  color: var(--muted-foreground);
  font-size: 0.6rem;
  cursor: default;
}

.quota-bar__legend-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}
</style>
