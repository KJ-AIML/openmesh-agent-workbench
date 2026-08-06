<script setup lang="ts">
import { ref, onMounted, computed, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { Minus, Square, X, LayoutGrid, MessageSquare, Mic, MicOff } from "lucide-vue-next";
import {
  minimizeWindow,
  toggleMaximizeWindow,
  closeWindow,
  startWindowDrag,
  isMaximized,
} from "../lib/adapters/windowAdapter";
import { isMacOS, isTauriRuntime, resolveIsMacOS } from "../lib/adapters/environment";
import { useStore } from "../lib/useStore";
import { useVoiceStore } from "../lib/voice/voiceStore";
import {
  toggleVoice,
  voicePttEnd,
  voicePttStart,
} from "../lib/voice/voiceSession";
import {
  enabledTopNavbarTabs,
  firstVisibleTopNavbarPath,
  normalizeAppearance,
  topNavbarTabForPath,
  type TopNavbarTabId,
} from "../lib/appearance";

const props = withDefaults(
  defineProps<{
    /** When sidebar is collapsed on macOS, clear native traffic lights. */
    clearanceForTrafficLights?: boolean;
  }>(),
  { clearanceForTrafficLights: false },
);

const { currentProject, settings } = useStore();
const route = useRoute();
const router = useRouter();

const appearance = computed(() =>
  normalizeAppearance(settings.value.appearance),
);

const hotTabs = computed(() => enabledTopNavbarTabs(appearance.value));

const TAB_ICONS: Partial<
  Record<TopNavbarTabId, typeof MessageSquare | typeof LayoutGrid>
> = {
  chat: MessageSquare,
  work: LayoutGrid,
};

watch(
  [() => route.path, hotTabs],
  ([path, tabs]) => {
    const current = topNavbarTabForPath(path);
    if (!current) return;
    if (tabs.some((t) => t.id === current.id)) return;
    void router.replace(firstVisibleTopNavbarPath(appearance.value));
  },
  { immediate: true },
);
const {
  enabled: voiceEnabled,
  phase: voicePhase,
  statusLabel: voiceStatusLabel,
  lastError: voiceLastError,
  listenMode: voiceListenMode,
} = useVoiceStore();
const voiceBusy = ref(false);
const pttHeld = ref(false);
/** Suppress the click that follows a PTT hold so we don't immediately toggle off. */
let suppressNextVoiceClick = false;

const macOS = ref(
  (window as unknown as { __OPENMESH_IS_MACOS__?: boolean }).__OPENMESH_IS_MACOS__ ??
    isMacOS(),
);
const maximized = ref(false);

/** Tabs share the ooo vertical band — spacer before nav, not just header padding. */
const showTrafficClearance = computed(
  () => macOS.value && props.clearanceForTrafficLights,
);

const titlebarClass = computed(() => [
  macOS.value ? "tb--mac" : "tb--win",
  showTrafficClearance.value ? "tb--mac-traffic-clearance" : null,
]);

const showCustomWindowControls = computed(
  () => isTauriRuntime() && !macOS.value,
);

const projectLabel = computed(() => currentProject.value?.name ?? null);

/** Edit page when a project is open; otherwise add/select flow. */
const projectNavTo = computed(() => {
  const p = currentProject.value;
  if (p?.id) return `/projects/${p.id}/edit`;
  return "/projects/new";
});

onMounted(async () => {
  macOS.value = await resolveIsMacOS();
  document.documentElement.dataset.platform = macOS.value ? "macos" : "other";
  document.documentElement.classList.toggle("is-macos", macOS.value);
  if (showCustomWindowControls.value) {
    maximized.value = await isMaximized();
  }
});

async function handleMinimize(e: MouseEvent) {
  e.stopPropagation();
  e.preventDefault();
  await minimizeWindow();
}

async function handleToggleMaximize(e: MouseEvent) {
  e.stopPropagation();
  e.preventDefault();
  await toggleMaximizeWindow();
  setTimeout(async () => {
    maximized.value = await isMaximized();
  }, 100);
}

async function handleClose(e: MouseEvent) {
  e.stopPropagation();
  e.preventDefault();
  await closeWindow();
}

async function handleVoiceClick(e: MouseEvent) {
  e.stopPropagation();
  e.preventDefault();
  if (suppressNextVoiceClick) {
    suppressNextVoiceClick = false;
    return;
  }
  if (voiceBusy.value) return;
  voiceBusy.value = true;
  try {
    await toggleVoice(router);
  } catch (err) {
    voicePhase.value = "error";
    voiceLastError.value =
      err instanceof Error ? err.message : "Could not start voice.";
    voiceEnabled.value = false;
  } finally {
    voiceBusy.value = false;
  }
}

async function handleVoicePointerDown(e: PointerEvent) {
  e.stopPropagation();
  if (voiceListenMode.value !== "ptt") return;
  suppressNextVoiceClick = true;
  if (!voiceEnabled.value) {
    voiceBusy.value = true;
    try {
      await toggleVoice(router);
    } finally {
      voiceBusy.value = false;
    }
  }
  if (!voiceEnabled.value) return;
  pttHeld.value = true;
  (e.currentTarget as HTMLElement | null)?.setPointerCapture?.(e.pointerId);
  await voicePttStart();
}

async function handleVoicePointerUp(e: PointerEvent) {
  e.stopPropagation();
  if (!pttHeld.value) return;
  pttHeld.value = false;
  await voicePttEnd(router);
}

const voiceTitle = computed(() => {
  if (!voiceEnabled.value) {
    return voiceListenMode.value === "ptt"
      ? "Hold mic to talk (push-to-talk)"
      : "Turn on Voice — speak freely, click again to stop";
  }
  return `Voice on — ${voiceStatusLabel.value}. Click mic to turn off.`;
});

async function handleDrag(e: MouseEvent) {
  if (e.button !== 0) return;
  const t = e.target as HTMLElement | null;
  if (t?.closest("a,button,input,textarea,select,[data-no-drag]")) return;
  e.preventDefault();
  await startWindowDrag();
}
</script>

<template>
  <!--
    macOS shell: titlebar sits ONLY on the main column (right of full-height sidebar).
    Traffic lights live over the sidebar top — same physical layer as the left rail.
  -->
  <header
    class="tb"
    :class="titlebarClass"
    data-tauri-drag-region
    @mousedown="handleDrag"
  >
    <!--
      Structural inset for Chat/Work/Docs/Sprint when the rail is gone.
      Padding alone on .tb lost to .tb--mac shorthand / felt flush at 78px;
      this flex spacer owns the tab cluster's left edge.
    -->
    <div
      v-if="showTrafficClearance"
      class="tb__traffic-clearance"
      aria-hidden="true"
      data-tauri-drag-region
    />

    <div v-if="!macOS" class="tb__brand" data-no-drag>
      <span class="tb__mark" aria-hidden="true">O</span>
      <span class="tb__name">OpenMesh</span>
    </div>

    <nav class="tb__nav" data-no-drag aria-label="Top navbar tabs">
      <router-link
        v-for="tab in hotTabs"
        :key="tab.id"
        :to="tab.path"
        class="tb__tab"
        :class="{ 'is-active': $route.path === tab.path }"
        :data-tab="tab.id"
      >
        <component
          :is="TAB_ICONS[tab.id]"
          v-if="TAB_ICONS[tab.id]"
          class="tb__tab-icon"
        />
        {{ tab.label }}
      </router-link>
    </nav>

    <div class="tb__spacer" data-tauri-drag-region />

    <div class="tb__end" data-no-drag>
      <button
        type="button"
        class="tb__voice"
        :class="{
          'is-on': voiceEnabled,
          'is-listening': voicePhase === 'listening' || pttHeld,
          'is-busy': voicePhase === 'thinking' || voicePhase === 'speaking',
        }"
        :title="voiceTitle"
        :aria-pressed="voiceEnabled"
        aria-label="OpenMesh Voice"
        @click="handleVoiceClick"
        @pointerdown="handleVoicePointerDown"
        @pointerup="handleVoicePointerUp"
        @pointercancel="handleVoicePointerUp"
      >
        <MicOff v-if="!voiceEnabled" class="h-3.5 w-3.5" />
        <Mic v-else class="h-3.5 w-3.5" />
      </button>
      <!-- Active project name on the right of the nav (replaces generic Projects tab) -->
      <router-link
        v-if="projectLabel"
        :to="projectNavTo"
        class="tb__project"
        :class="{ 'is-active': $route.path.startsWith('/projects') }"
        :title="currentProject?.folderPath || projectLabel"
      >
        <span class="tb__project-dot" aria-hidden="true" />
        <span class="tb__project-name">{{ projectLabel }}</span>
      </router-link>
      <router-link
        v-else
        to="/projects/new"
        class="tb__tab tb__tab--ghost"
        :class="{ 'is-active': $route.path.startsWith('/projects') }"
      >
        Projects
      </router-link>
      <span class="tb__badge">Desktop</span>
      <template v-if="showCustomWindowControls">
        <button class="tb__winbtn" title="Minimize" aria-label="Minimize" @click="handleMinimize">
          <Minus class="h-3.5 w-3.5" />
        </button>
        <button
          class="tb__winbtn"
          :title="maximized ? 'Restore' : 'Maximize'"
          @click="handleToggleMaximize"
        >
          <Square class="h-3 w-3" />
        </button>
        <button class="tb__winbtn tb__winbtn--close" title="Close" @click="handleClose">
          <X class="h-3.5 w-3.5" />
        </button>
      </template>
    </div>
  </header>
</template>

<style scoped>
.tb {
  z-index: 50;
  box-sizing: border-box;
  height: 40px;
  min-height: 40px;
  background: var(--sidebar);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 12px;
  user-select: none;
  -webkit-user-select: none;
}

.tb--mac {
  /* Match sidebar-mac-top exactly — one continuous top seam */
  height: var(--chrome-top, 44px);
  min-height: var(--chrome-top, 44px);
  padding: 0 14px 0 12px;
  background: var(--sidebar);
  border-bottom: 1px solid var(--border);
}

/* Sidebar collapsed: spacer owns left inset — zero header pad so widths don't stack. */
.tb--mac.tb--mac-traffic-clearance {
  padding-left: 0;
}

.tb__traffic-clearance {
  flex: 0 0 var(--mac-traffic-lights-inset, 96px);
  width: var(--mac-traffic-lights-inset, 96px);
  min-width: var(--mac-traffic-lights-inset, 96px);
  align-self: stretch;
  pointer-events: none;
}

.tb__brand {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.tb__mark {
  width: 18px;
  height: 18px;
  border-radius: 5px;
  display: grid;
  place-items: center;
  font-size: 10px;
  font-weight: 700;
  background: var(--foreground);
  color: var(--background);
}

.tb__name {
  font-size: 13px;
  font-weight: 600;
  letter-spacing: -0.02em;
  color: var(--foreground);
}

.tb__nav {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}

.tb__tab {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 28px;
  padding: 0 11px;
  border-radius: 7px;
  font-size: 12px;
  font-weight: 500;
  color: var(--muted-foreground);
  text-decoration: none;
  white-space: nowrap;
  transition: background 0.12s ease, color 0.12s ease;
}

.tb__tab-icon {
  width: 12px;
  height: 12px;
  opacity: 0.85;
}

.tb__tab:hover {
  color: var(--foreground);
  background: var(--surface-highlight);
}

.tb__tab.is-active {
  color: var(--foreground);
  background: var(--surface-3);
  font-weight: 600;
}

.tb__tab.is-active .tb__tab-icon {
  opacity: 1;
}

.tb__spacer {
  flex: 1;
  min-width: 12px;
  height: 100%;
}

.tb__end {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  min-width: 0;
}

.tb__voice {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border-radius: 7px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--muted-foreground);
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease, border-color 0.12s ease;
}

.tb__voice:hover {
  color: var(--foreground);
  background: var(--surface-3);
}

.tb__voice.is-on {
  color: var(--foreground);
  border-color: color-mix(in oklab, #3d9a6a 45%, var(--border));
  background: color-mix(in oklab, #3d9a6a 18%, var(--surface-2));
}

.tb__voice.is-listening {
  animation: tb-voice-pulse 1.2s ease-in-out infinite;
}

.tb__voice.is-busy {
  border-color: color-mix(in oklab, #c9a227 40%, var(--border));
}

@keyframes tb-voice-pulse {
  0%,
  100% {
    box-shadow: 0 0 0 0 color-mix(in oklab, #3d9a6a 0%, transparent);
  }
  50% {
    box-shadow: 0 0 0 4px color-mix(in oklab, #3d9a6a 22%, transparent);
  }
}

.tb__project {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: 200px;
  min-width: 0;
  height: 28px;
  padding: 0 10px;
  border-radius: 7px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  text-decoration: none;
  color: inherit;
  transition: background 0.12s ease, border-color 0.12s ease;
  cursor: pointer;
}

.tb__project:hover {
  background: var(--surface-3);
  border-color: var(--border-strong, var(--border));
}

.tb__project.is-active {
  background: var(--surface-3);
  border-color: var(--border-strong, var(--border));
}

.tb__project-dot {
  width: 6px;
  height: 6px;
  border-radius: 999px;
  background: var(--accent-green);
  flex-shrink: 0;
}

.tb__project-name {
  font-size: 12px;
  font-weight: 600;
  letter-spacing: -0.01em;
  color: var(--foreground);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  line-height: 1;
}

.tb__tab--ghost {
  /* same as tab; used when no project selected */
}

.tb__badge {
  font-size: 10px;
  font-weight: 600;
  line-height: 1;
  padding: 5px 9px;
  border-radius: 999px;
  color: #4ade80;
  background: rgba(34, 197, 94, 0.12);
  border: 1px solid rgba(34, 197, 94, 0.28);
  flex-shrink: 0;
}

.tb__winbtn {
  width: 36px;
  height: 28px;
  display: grid;
  place-items: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--muted-foreground);
  cursor: pointer;
}

.tb__winbtn:hover {
  background: var(--surface-hover);
  color: var(--foreground);
}

.tb__winbtn--close:hover {
  background: var(--accent-red);
  color: #fff;
}
</style>
