<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { Undo2, X } from "lucide-vue-next";
import {
  listActionAudit,
  peekUndoIntent,
  subscribeAudit,
} from "../../lib/appActions/audit";
import { labelForAction } from "../../lib/appActions/types";

const emit = defineEmits<{
  undo: [];
  hide: [];
}>();

const tick = ref(0);
let unsub: (() => void) | null = null;

onMounted(() => {
  unsub = subscribeAudit(() => {
    tick.value += 1;
  });
});
onUnmounted(() => unsub?.());

const entries = computed(() => {
  tick.value;
  return listActionAudit(8);
});
const canUndo = computed(() => {
  tick.value;
  return !!peekUndoIntent();
});
</script>

<template>
  <aside class="action-run">
    <header class="action-run__head">
      <span>Action trail</span>
      <div class="action-run__actions">
        <button
          type="button"
          class="btn-secondary action-run__undo"
          :disabled="!canUndo"
          @click="emit('undo')"
        >
          <Undo2 :size="13" /> Undo
        </button>
        <button
          type="button"
          class="action-run__hide"
          title="Hide action trail"
          aria-label="Hide action trail"
          @click="emit('hide')"
        >
          <X :size="14" />
        </button>
      </div>
    </header>
    <ul v-if="entries.length" class="action-run__list">
      <li v-for="e in entries" :key="e.id" :class="{ 'is-fail': !e.ok }">
        <span class="action-run__sum">{{ e.summary || labelForAction(e.action) }}</span>
        <span class="action-run__src">{{ e.source }}</span>
      </li>
    </ul>
    <p v-else class="action-run__empty">No actions yet.</p>
  </aside>
</template>

<style scoped>
.action-run {
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 0.65rem 0.75rem;
  background: var(--surface);
  font-size: 0.78rem;
  /* Wrapper uses pointer-events: none; only the card captures. */
  pointer-events: auto;
}
.action-run__head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.5rem;
  font-weight: 600;
  margin-bottom: 0.45rem;
}
.action-run__actions {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  flex-shrink: 0;
}
.action-run__undo {
  font-size: 0.72rem;
  padding: 0.2rem 0.45rem;
}
.action-run__hide {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  height: 1.5rem;
  padding: 0;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: inherit;
  opacity: 0.65;
  cursor: pointer;
}
.action-run__hide:hover {
  opacity: 1;
  background: color-mix(in oklab, var(--border) 55%, transparent);
}
.action-run__list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 0.35rem;
}
.action-run__list li {
  display: flex;
  justify-content: space-between;
  gap: 0.5rem;
  opacity: 0.9;
}
.action-run__list li.is-fail {
  color: var(--danger, #c45);
}
.action-run__src {
  opacity: 0.55;
  text-transform: uppercase;
  font-size: 0.65rem;
}
.action-run__empty {
  margin: 0;
  opacity: 0.55;
}
</style>
