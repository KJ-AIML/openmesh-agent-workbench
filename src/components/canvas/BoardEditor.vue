<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { BoardScene } from "../../lib/canvas/boards";
import {
  mountExcalidrawIsland,
  type ExcalidrawIslandHandle,
} from "../../lib/canvas/excalidrawIsland";

const props = defineProps<{
  /** Remount key — change when switching boards. */
  boardId: string;
  scene: BoardScene | null;
  theme?: "light" | "dark";
}>();

const emit = defineEmits<{
  change: [scene: BoardScene];
}>();

const host = ref<HTMLDivElement | null>(null);
let island: ExcalidrawIslandHandle | null = null;
let changeTimer: ReturnType<typeof setTimeout> | null = null;

function detectTheme(): "light" | "dark" {
  if (props.theme) return props.theme;
  return document.documentElement.classList.contains("dark") ? "dark" : "light";
}

function mount() {
  if (!host.value) return;
  island?.unmount();
  island = mountExcalidrawIsland(host.value, {
    initialScene: props.scene,
    theme: detectTheme(),
    onChange: (scene) => {
      if (changeTimer) clearTimeout(changeTimer);
      // Debounce persistence — Excalidraw onChange is high-frequency.
      changeTimer = setTimeout(() => emit("change", scene), 450);
    },
  });
}

onMounted(mount);

watch(
  () => props.boardId,
  () => mount(),
);

onBeforeUnmount(() => {
  if (changeTimer) clearTimeout(changeTimer);
  island?.unmount();
  island = null;
});
</script>

<template>
  <div class="board-editor">
    <div ref="host" class="board-editor__host" />
  </div>
</template>

<style scoped>
.board-editor {
  position: relative;
  width: 100%;
  height: min(68vh, 720px);
  min-height: 420px;
  overflow: hidden;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--surface-2);
}

.board-editor__host {
  width: 100%;
  height: 100%;
}

.board-editor__host :deep(.excalidraw) {
  --color-primary: var(--accent-blue);
  height: 100%;
}
</style>
