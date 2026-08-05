<script setup lang="ts">
import { Check, X } from "lucide-vue-next";
import type { PendingAction } from "../../lib/appActions/pending";
import { labelForAction } from "../../lib/appActions/types";

defineProps<{
  item: PendingAction;
}>();

const emit = defineEmits<{
  approve: [id: string];
  reject: [id: string];
}>();
</script>

<template>
  <div class="action-approval" :data-policy="item.policy">
    <div class="action-approval__body">
      <p class="action-approval__label">{{ item.label || labelForAction(item.intent.action) }}</p>
      <p class="action-approval__meta">
        {{ item.policy === "hard" ? "Needs explicit approval" : "Confirm to continue" }}
        · {{ item.intent.source }}
      </p>
    </div>
    <div class="action-approval__ops">
      <button type="button" class="btn-secondary" @click="emit('reject', item.id)">
        <X :size="14" /> Reject
      </button>
      <button type="button" class="btn-primary" @click="emit('approve', item.id)">
        <Check :size="14" /> Approve
      </button>
    </div>
  </div>
</template>

<style scoped>
.action-approval {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 0.9rem;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-2);
}
.action-approval[data-policy="hard"] {
  border-color: color-mix(in srgb, var(--danger, #c45) 55%, var(--border));
}
.action-approval__label {
  margin: 0;
  font-weight: 600;
  font-size: 0.9rem;
}
.action-approval__meta {
  margin: 0.2rem 0 0;
  font-size: 0.75rem;
  opacity: 0.7;
}
.action-approval__ops {
  display: flex;
  gap: 0.45rem;
}
</style>
