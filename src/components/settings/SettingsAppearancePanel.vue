<script setup lang="ts">
import { computed, ref } from "vue";
import { useStore } from "../../lib/useStore";
import {
  applyAppearance,
  normalizeAppearance,
  normalizeTopNavbarTabs,
  TOP_NAVBAR_TAB_DEFS,
  type AppearanceDensity,
  type AppearanceFontSize,
  type AppearancePrefs,
  type AppearanceTheme,
  type TopNavbarTabId,
} from "../../lib/appearance";

const { settings, saveSettings } = useStore();
const status = ref("");

const appearance = computed(() =>
  normalizeAppearance(settings.value.appearance),
);

const enabledTabCount = computed(
  () =>
    TOP_NAVBAR_TAB_DEFS.filter((t) => appearance.value.topNavbarTabs[t.id])
      .length,
);

async function commit(next: AppearancePrefs) {
  applyAppearance(next);
  await saveSettings({
    appearance: next,
    workspace: {
      ...settings.value.workspace,
      theme: next.theme,
    },
  });
  status.value = "Saved";
  window.setTimeout(() => {
    if (status.value === "Saved") status.value = "";
  }, 1600);
}

function setTheme(theme: AppearanceTheme) {
  void commit({ ...appearance.value, theme });
}

function setFontSize(fontSize: AppearanceFontSize) {
  void commit({ ...appearance.value, fontSize });
}

function setDensity(density: AppearanceDensity) {
  void commit({ ...appearance.value, density });
}

function toggleTopNavbarTab(id: TopNavbarTabId) {
  const current = appearance.value.topNavbarTabs;
  const turningOff = current[id];
  if (turningOff && enabledTabCount.value <= 1) return;
  const nextTabs = normalizeTopNavbarTabs({ ...current, [id]: !current[id] });
  void commit({ ...appearance.value, topNavbarTabs: nextTabs });
}

const themes: { id: AppearanceTheme; label: string; hint: string }[] = [
  { id: "dark", label: "Dark", hint: "Default workbench" },
  { id: "light", label: "Light", hint: "Bright surfaces" },
  { id: "system", label: "System", hint: "Follow OS" },
];

const fontSizes: { id: AppearanceFontSize; label: string }[] = [
  { id: "small", label: "Small" },
  { id: "medium", label: "Medium" },
  { id: "large", label: "Large" },
];

const densities: { id: AppearanceDensity; label: string; hint: string }[] = [
  { id: "comfortable", label: "Comfortable", hint: "Roomier chrome" },
  { id: "compact", label: "Compact", hint: "Tighter sidebar" },
];
</script>

<template>
  <div class="workbench-card p-5 space-y-5 appearance-panel">
    <div class="appearance-panel__head">
      <div>
        <p class="appearance-panel__title">Appearance</p>
        <p class="appearance-panel__desc">
          Theme, type size, and chrome density. Changes apply immediately.
        </p>
      </div>
      <span
        v-if="status"
        class="appearance-panel__status"
        role="status"
        >{{ status }}</span
      >
    </div>

    <div
      class="appearance-preview"
      aria-label="Live theme preview"
      data-testid="appearance-preview"
    >
      <div class="appearance-preview__swatches">
        <span class="appearance-preview__swatch appearance-preview__swatch--bg" title="Background" />
        <span class="appearance-preview__swatch appearance-preview__swatch--surface" title="Surface" />
        <span class="appearance-preview__swatch appearance-preview__swatch--text" title="Text" />
        <span class="appearance-preview__swatch appearance-preview__swatch--accent" title="Accent" />
      </div>
      <div class="appearance-preview__copy">
        <p class="appearance-preview__title">OpenMesh</p>
        <p class="appearance-preview__sample">
          Sidebar · Chat · Continuity — tokens update with your selection.
        </p>
      </div>
    </div>

    <div>
      <label class="appearance-panel__label">Theme</label>
      <div class="om-seg appearance-panel__seg" role="radiogroup" aria-label="Theme">
        <button
          v-for="t in themes"
          :key="t.id"
          type="button"
          role="radio"
          class="om-seg__btn"
          :class="{ 'is-active': appearance.theme === t.id }"
          :aria-checked="appearance.theme === t.id"
          :title="t.hint"
          @click="setTheme(t.id)"
        >
          {{ t.label }}
        </button>
      </div>
    </div>

    <div>
      <label class="appearance-panel__label">Font size</label>
      <div class="om-seg appearance-panel__seg" role="radiogroup" aria-label="Font size">
        <button
          v-for="f in fontSizes"
          :key="f.id"
          type="button"
          role="radio"
          class="om-seg__btn"
          :class="{ 'is-active': appearance.fontSize === f.id }"
          :aria-checked="appearance.fontSize === f.id"
          @click="setFontSize(f.id)"
        >
          {{ f.label }}
        </button>
      </div>
    </div>

    <div>
      <label class="appearance-panel__label">Density</label>
      <div class="om-seg appearance-panel__seg" role="radiogroup" aria-label="Density">
        <button
          v-for="d in densities"
          :key="d.id"
          type="button"
          role="radio"
          class="om-seg__btn"
          :class="{ 'is-active': appearance.density === d.id }"
          :aria-checked="appearance.density === d.id"
          :title="d.hint"
          @click="setDensity(d.id)"
        >
          {{ d.label }}
        </button>
      </div>
      <p class="appearance-panel__hint text-caption text-muted">
        Compact shortens sidebar rows and section gaps.
      </p>
    </div>

    <div>
      <label class="appearance-panel__label">Top navbar tabs</label>
      <div
        class="om-seg appearance-panel__seg appearance-panel__tabs"
        role="group"
        aria-label="Top navbar tabs"
        data-testid="appearance-top-navbar-tabs"
      >
        <button
          v-for="tab in TOP_NAVBAR_TAB_DEFS"
          :key="tab.id"
          type="button"
          class="om-seg__btn"
          :class="{ 'is-active': appearance.topNavbarTabs[tab.id] }"
          :aria-pressed="appearance.topNavbarTabs[tab.id]"
          :disabled="
            appearance.topNavbarTabs[tab.id] && enabledTabCount <= 1
          "
          :title="
            appearance.topNavbarTabs[tab.id] && enabledTabCount <= 1
              ? 'Keep at least one tab visible'
              : `Show ${tab.label} in the top navbar`
          "
          @click="toggleTopNavbarTab(tab.id)"
        >
          {{ tab.label }}
        </button>
      </div>
      <p class="appearance-panel__hint text-caption text-muted">
        Choose which topics appear in the titlebar. Sidebar still lists all
        pages. At least one tab stays on.
      </p>
    </div>
  </div>
</template>

<style scoped>
.appearance-panel__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
}

.appearance-panel__title {
  margin: 0;
  font-size: 0.92rem;
  font-weight: 600;
  letter-spacing: -0.015em;
}

.appearance-panel__desc {
  margin: 0.25rem 0 0;
  font-size: 0.78rem;
  color: var(--muted-foreground);
}

.appearance-panel__status {
  flex-shrink: 0;
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: -0.01em;
  color: var(--accent-green);
  padding-top: 0.15rem;
}

.appearance-panel__label {
  display: block;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--muted-foreground);
  margin-bottom: 0.5rem;
  letter-spacing: -0.005em;
}

.appearance-panel__seg {
  width: 100%;
  max-width: 28rem;
}

.appearance-panel__seg .om-seg__btn {
  flex: 1;
  text-align: center;
}

.appearance-panel__hint {
  margin-top: 0.45rem;
}

.appearance-preview {
  display: flex;
  align-items: center;
  gap: 0.9rem;
  padding: 0.85rem 1rem;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--surface-1, var(--muted));
}

.appearance-preview__swatches {
  display: flex;
  gap: 0.35rem;
  flex-shrink: 0;
}

.appearance-preview__swatch {
  width: 1.35rem;
  height: 1.35rem;
  border-radius: 6px;
  border: 1px solid var(--border-strong, var(--border));
}

.appearance-preview__swatch--bg {
  background: var(--background);
}

.appearance-preview__swatch--surface {
  background: var(--surface-2, var(--card));
}

.appearance-preview__swatch--text {
  background: var(--foreground);
}

.appearance-preview__swatch--accent {
  background: var(--accent-blue);
}

.appearance-preview__title {
  font-size: 0.8125rem;
  font-weight: 600;
  letter-spacing: -0.015em;
  color: var(--foreground);
  line-height: 1.2;
}

.appearance-preview__sample {
  margin-top: 0.15rem;
  font-size: 0.72rem;
  color: var(--muted-foreground);
  letter-spacing: -0.01em;
  line-height: 1.35;
}
</style>
