<script setup lang="ts">
import type { AutoUiBlock, AutoUiDocument } from "../../lib/canvas/autoUi";

defineProps<{
  doc: AutoUiDocument;
}>();

function pillClass(tone?: string): string {
  const t = (tone || "").toLowerCase();
  if (t === "good" || t === "success" || t === "green") return "omc-pill--good";
  if (t === "warn" || t === "amber" || t === "warning") return "omc-pill--warn";
  if (t === "bad" || t === "danger" || t === "red") return "omc-pill--bad";
  return "omc-pill--neutral";
}

function calloutClass(tone?: string): string {
  const t = (tone || "info").toLowerCase();
  if (t === "warn" || t === "warning") return "omc-callout--warn";
  if (t === "bad" || t === "danger" || t === "error") return "omc-callout--bad";
  if (t === "good" || t === "success") return "omc-callout--good";
  return "omc-callout--info";
}

function blockKey(b: AutoUiBlock, i: number): string {
  return `${b.type}-${i}`;
}
</script>

<template>
  <article class="omc">
    <header class="omc__head">
      <h1 class="omc__title">{{ doc.title }}</h1>
      <p v-if="doc.summary" class="omc__summary">{{ doc.summary }}</p>
    </header>

    <div class="omc__stack">
      <template v-for="(block, i) in doc.blocks" :key="blockKey(block, i)">
        <h1 v-if="block.type === 'h1'" class="omc__h1">{{ block.text }}</h1>
        <h2 v-else-if="block.type === 'h2'" class="omc__h2">{{ block.text }}</h2>
        <p v-else-if="block.type === 'text'" class="omc__text">{{ block.text }}</p>
        <div
          v-else-if="block.type === 'callout'"
          class="omc-callout"
          :class="calloutClass(block.tone)"
        >
          {{ block.text }}
        </div>
        <div v-else-if="block.type === 'stat'" class="omc-stat">
          <span class="omc-stat__label">{{ block.label }}</span>
          <span class="omc-stat__value">{{ block.value }}</span>
          <span v-if="block.hint" class="omc-stat__hint">{{ block.hint }}</span>
        </div>
        <div v-else-if="block.type === 'stats'" class="omc-stats">
          <div v-for="(item, j) in block.items" :key="j" class="omc-stat">
            <span class="omc-stat__label">{{ item.label }}</span>
            <span class="omc-stat__value">{{ item.value }}</span>
            <span v-if="item.hint" class="omc-stat__hint">{{ item.hint }}</span>
          </div>
        </div>
        <div v-else-if="block.type === 'table'" class="omc-table-wrap">
          <table class="omc-table">
            <thead>
              <tr>
                <th v-for="(col, ci) in block.columns" :key="ci">{{ col }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(row, ri) in block.rows" :key="ri">
                <td v-for="(cell, ci) in row" :key="ci">{{ cell }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-else-if="block.type === 'pills'" class="omc-pills">
          <span
            v-for="(p, pi) in block.items"
            :key="pi"
            class="omc-pill"
            :class="pillClass(p.tone)"
          >
            {{ p.text }}
          </span>
        </div>
        <ul v-else-if="block.type === 'todo'" class="omc-todo">
          <li v-for="(t, ti) in block.items" :key="ti" :class="{ 'omc-todo--done': t.done }">
            <span class="omc-todo__mark">{{ t.done ? "✓" : "○" }}</span>
            {{ t.text }}
          </li>
        </ul>
        <pre v-else-if="block.type === 'code'" class="omc-code"><code>{{ block.code }}</code></pre>
        <hr v-else-if="block.type === 'divider'" class="omc-divider" />
      </template>
    </div>
  </article>
</template>

<style scoped>
.omc {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  max-width: 920px;
}

.omc__head {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.omc__title {
  margin: 0;
  font-size: 1.35rem;
  font-weight: 650;
  letter-spacing: -0.02em;
  color: var(--foreground);
}

.omc__summary {
  margin: 0;
  font-size: 0.875rem;
  color: var(--muted-foreground);
  line-height: 1.45;
}

.omc__stack {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.omc__h1 {
  margin: 0.25rem 0 0;
  font-size: 1.15rem;
  font-weight: 650;
}

.omc__h2 {
  margin: 0.15rem 0 0;
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--muted-foreground);
}

.omc__text {
  margin: 0;
  font-size: 0.9rem;
  line-height: 1.55;
  color: var(--foreground);
}

.omc-callout {
  padding: 0.75rem 0.9rem;
  border-radius: 12px;
  border: 1px solid var(--border);
  font-size: 0.875rem;
  line-height: 1.45;
  background: var(--surface-2);
}

.omc-callout--info {
  border-color: color-mix(in srgb, var(--accent-blue) 35%, var(--border));
}
.omc-callout--warn {
  border-color: color-mix(in srgb, var(--accent-amber) 40%, var(--border));
}
.omc-callout--bad {
  border-color: color-mix(in srgb, var(--accent-red) 40%, var(--border));
}
.omc-callout--good {
  border-color: color-mix(in srgb, var(--accent-green) 40%, var(--border));
}

.omc-stats {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
  gap: 0.65rem;
}

.omc-stat {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  padding: 0.85rem 0.95rem;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--surface-1);
}

.omc-stat__label {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted-foreground);
}

.omc-stat__value {
  font-size: 1.25rem;
  font-weight: 650;
  letter-spacing: -0.02em;
}

.omc-stat__hint {
  font-size: 0.75rem;
  color: var(--muted-foreground);
}

.omc-table-wrap {
  overflow: auto;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--surface-1);
}

.omc-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.8125rem;
}

.omc-table th,
.omc-table td {
  padding: 0.55rem 0.75rem;
  text-align: left;
  border-bottom: 1px solid var(--border);
}

.omc-table th {
  color: var(--muted-foreground);
  font-weight: 550;
  background: var(--surface-2);
}

.omc-pills {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}

.omc-pill {
  display: inline-flex;
  align-items: center;
  padding: 0.2rem 0.55rem;
  border-radius: 999px;
  border: 1px solid var(--border);
  font-size: 0.75rem;
  background: var(--surface-2);
}

.omc-pill--good {
  color: var(--accent-green);
}
.omc-pill--warn {
  color: var(--accent-amber);
}
.omc-pill--bad {
  color: var(--accent-red);
}

.omc-todo {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  font-size: 0.875rem;
}

.omc-todo__mark {
  display: inline-block;
  width: 1.1rem;
  color: var(--muted-foreground);
}

.omc-todo--done {
  color: var(--muted-foreground);
  text-decoration: line-through;
}

.omc-code {
  margin: 0;
  padding: 0.85rem 1rem;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  font-family: var(--font-mono);
  font-size: 0.78rem;
  overflow: auto;
  white-space: pre-wrap;
}

.omc-divider {
  border: none;
  border-top: 1px solid var(--border);
  margin: 0.25rem 0;
}
</style>
