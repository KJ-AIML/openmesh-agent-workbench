<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  Plus,
  Link2,
  Maximize2,
  Trash2,
  LayoutTemplate,
  Pencil,
  Save,
} from "lucide-vue-next";
import { useStore } from "../lib/useStore";
import { useCanvasStore } from "../lib/canvas/store";
import {
  deleteAutoUi,
  listAutoUi,
  type AutoUiDocument,
} from "../lib/canvas/autoUi";
import {
  createBoard,
  deleteBoard,
  listBoards,
  saveBoardScene,
  type BoardDocument,
  type BoardScene,
} from "../lib/canvas/boards";
import OmCanvasRenderer from "../components/canvas/OmCanvasRenderer.vue";
import BoardEditor from "../components/canvas/BoardEditor.vue";
import { registerAppActionHandlers } from "../lib/appActions/dispatcher";
import { setAppContext } from "../lib/appActions/context";

type TabId = "network" | "auto-ui" | "board";

const router = useRouter();
const route = useRoute();
const { currentProjectPath } = useStore();
const canvas = useCanvasStore();

const tab = ref<TabId>("auto-ui");
const artifacts = ref<AutoUiDocument[]>([]);
const activeArtifactId = ref<string | null>(null);
const autoUiError = ref<string | null>(null);

const boards = ref<BoardDocument[]>([]);
const activeBoardId = ref<string | null>(null);
/** Bump to remount Excalidraw after external AppAction scene edits. */
const boardEpoch = ref(0);
const boardError = ref<string | null>(null);
const boardSaving = ref(false);
const boardDirty = ref(false);
const boardStatus = ref<string | null>(null);

const nodes = computed(() => canvas.active.value?.nodes ?? []);
const edges = computed(() => canvas.active.value?.edges ?? []);
const activeArtifact = computed(
  () => artifacts.value.find((a) => a.id === activeArtifactId.value) ?? null,
);
const activeBoard = computed(
  () => boards.value.find((b) => b.id === activeBoardId.value) ?? null,
);

const viewBox = computed(() => {
  canvas.fitToken.value;
  if (!nodes.value.length) return "0 0 900 560";
  const xs = nodes.value.map((n) => n.x);
  const ys = nodes.value.map((n) => n.y);
  const minX = Math.min(...xs) - 40;
  const minY = Math.min(...ys) - 40;
  const maxX = Math.max(...xs) + 180;
  const maxY = Math.max(...ys) + 100;
  return `${minX} ${minY} ${Math.max(400, maxX - minX)} ${Math.max(320, maxY - minY)}`;
});

function applyTabFromQuery() {
  const q = route.query.tab;
  if (q === "board" || q === "network" || q === "auto-ui") {
    tab.value = q;
  }
}

async function refreshArtifacts(path: string) {
  try {
    artifacts.value = await listAutoUi(path);
    if (
      activeArtifactId.value &&
      !artifacts.value.some((a) => a.id === activeArtifactId.value)
    ) {
      activeArtifactId.value = artifacts.value[0]?.id ?? null;
    } else if (!activeArtifactId.value && artifacts.value[0]) {
      activeArtifactId.value = artifacts.value[0].id;
    }
    autoUiError.value = null;
  } catch (e) {
    autoUiError.value = e instanceof Error ? e.message : String(e);
  }
}

async function refreshBoards(path: string) {
  try {
    boards.value = await listBoards(path);
    if (
      activeBoardId.value &&
      !boards.value.some((b) => b.id === activeBoardId.value)
    ) {
      activeBoardId.value = boards.value[0]?.id ?? null;
    } else if (!activeBoardId.value && boards.value[0]) {
      activeBoardId.value = boards.value[0].id;
    }
    boardError.value = null;
  } catch (e) {
    boardError.value = e instanceof Error ? e.message : String(e);
  }
}

async function openBoardSurface(boardId?: string) {
  const path = currentProjectPath.value;
  tab.value = "board";
  await router.push({ path: "/canvas", query: { tab: "board" } });
  if (!path) return;
  await refreshBoards(path);
  if (boardId) {
    activeBoardId.value = boardId;
  } else if (!activeBoardId.value) {
    await newBoard();
  }
}

onMounted(async () => {
  applyTabFromQuery();
  const path = currentProjectPath.value;
  if (!path) return;
  registerAppActionHandlers({
    openCanvas: async (id) => {
      if (id) await canvas.load(path, id);
      else await canvas.ensureCanvas(path);
      tab.value = "network";
      await router.push("/canvas");
    },
    openBoard: (id) => openBoardSurface(id),
    boardAddSticky: async (text, boardId) => {
      const { boardAddSticky } = await import("../lib/canvas/boards");
      const doc = await boardAddSticky(path, text, boardId ?? activeBoardId.value ?? undefined);
      activeBoardId.value = doc.id;
      tab.value = "board";
      await refreshBoards(path);
      boardEpoch.value += 1;
      return `Sticky on ${doc.title}`;
    },
    boardConnect: async (from, to, boardId) => {
      const { boardConnect } = await import("../lib/canvas/boards");
      const doc = await boardConnect(
        path,
        from,
        to,
        boardId ?? activeBoardId.value ?? undefined,
      );
      activeBoardId.value = doc.id;
      tab.value = "board";
      await refreshBoards(path);
      boardEpoch.value += 1;
      return `Connected ${from} → ${to}`;
    },
    canvasAddNode: (label, kind) => canvas.addNode(path, label, kind),
    canvasConnect: (from, to) => canvas.connect(path, from, to),
    canvasDeleteNode: (id) => canvas.deleteNode(path, id),
    canvasFitView: () => canvas.requestFitView(),
  });
  setAppContext({
    route: "/canvas",
    projectPath: path,
    canvasId: canvas.active.value?.id,
  });
  await canvas.refreshList(path);
  if (!canvas.active.value) await canvas.ensureCanvas(path);
  await refreshArtifacts(path);
  await refreshBoards(path);
});

watch(
  () => canvas.active.value?.id,
  (id) => setAppContext({ canvasId: id, route: "/canvas" }),
);

watch(
  () => route.query.tab,
  () => applyTabFromQuery(),
);

watch(currentProjectPath, async (path) => {
  if (!path) return;
  await canvas.refreshList(path);
  if (!canvas.active.value) await canvas.ensureCanvas(path);
  await refreshArtifacts(path);
  await refreshBoards(path);
});

async function addMachine() {
  const path = currentProjectPath.value;
  if (!path) return;
  const n = (canvas.active.value?.nodes.length ?? 0) + 1;
  await canvas.addNode(path, `Machine ${n}`, "machine");
}

async function connectLastTwo() {
  const path = currentProjectPath.value;
  const ns = canvas.active.value?.nodes ?? [];
  if (!path || ns.length < 2) return;
  await canvas.connect(path, ns[ns.length - 2].id, ns[ns.length - 1].id);
}

async function removeArtifact(id: string) {
  const path = currentProjectPath.value;
  if (!path) return;
  await deleteAutoUi(path, id);
  await refreshArtifacts(path);
}

async function newBoard() {
  const path = currentProjectPath.value;
  if (!path) return;
  try {
    const n = boards.value.length + 1;
    const doc = await createBoard(path, `Board ${n}`);
    await refreshBoards(path);
    activeBoardId.value = doc.id;
    boardDirty.value = false;
    boardStatus.value = "Created";
    boardError.value = null;
  } catch (e) {
    boardError.value = e instanceof Error ? e.message : String(e);
  }
}

async function removeBoard(id: string) {
  const path = currentProjectPath.value;
  if (!path) return;
  try {
    await deleteBoard(path, id);
    if (activeBoardId.value === id) activeBoardId.value = null;
    await refreshBoards(path);
    boardDirty.value = false;
    boardStatus.value = "Deleted";
  } catch (e) {
    boardError.value = e instanceof Error ? e.message : String(e);
  }
}

async function onBoardSceneChange(scene: BoardScene) {
  const path = currentProjectPath.value;
  const id = activeBoardId.value;
  if (!path || !id) return;
  boardDirty.value = true;
  boardSaving.value = true;
  try {
    const saved = await saveBoardScene(path, id, scene);
    const idx = boards.value.findIndex((b) => b.id === id);
    if (idx >= 0) boards.value[idx] = saved;
    boardDirty.value = false;
    boardStatus.value = "Saved";
    boardError.value = null;
  } catch (e) {
    boardError.value = e instanceof Error ? e.message : String(e);
  } finally {
    boardSaving.value = false;
  }
}

function relativeTime(ms: number): string {
  const d = Date.now() - ms;
  if (d < 60_000) return "just now";
  if (d < 3_600_000) return `${Math.floor(d / 60_000)}m ago`;
  if (d < 86_400_000) return `${Math.floor(d / 3_600_000)}h ago`;
  return `${Math.floor(d / 86_400_000)}d ago`;
}
</script>

<template>
  <div class="cv">
    <header class="cv__head">
      <div>
        <h1 class="cv__title">Canvas</h1>
        <p class="cv__meta">
          <strong>Auto UI</strong> (agent JSON boards), freeform
          <strong>Board</strong> (draw/edit), and <strong>Network</strong>
          graph — three surfaces, one Canvas page.
        </p>
      </div>
      <div class="om-seg" role="tablist" aria-label="Canvas mode">
        <button
          type="button"
          class="om-seg__btn"
          :class="{ 'is-active': tab === 'auto-ui' }"
          role="tab"
          @click="tab = 'auto-ui'"
        >
          Auto UI
        </button>
        <button
          type="button"
          class="om-seg__btn"
          :class="{ 'is-active': tab === 'board' }"
          role="tab"
          @click="tab = 'board'"
        >
          Board
        </button>
        <button
          type="button"
          class="om-seg__btn"
          :class="{ 'is-active': tab === 'network' }"
          role="tab"
          @click="tab = 'network'"
        >
          Network
        </button>
      </div>
    </header>

    <p v-if="!currentProjectPath" class="cv__empty">Open a project to use Canvas.</p>

    <!-- Auto UI -->
    <div v-else-if="tab === 'auto-ui'" class="cv__auto">
      <aside class="cv__rail workbench-card">
        <div class="cv__rail-head">
          <LayoutTemplate :size="14" />
          <span>Artifacts</span>
        </div>
        <p v-if="autoUiError" class="cv__empty">{{ autoUiError }}</p>
        <p v-else-if="!artifacts.length" class="cv__rail-empty">
          No Auto UI boards yet. In Chat, ask for an Auto UI board — or save a
          <code>openmesh.canvas/1</code> document from a chat fence.
        </p>
        <button
          v-for="a in artifacts"
          :key="a.id"
          type="button"
          class="cv__rail-item"
          :class="{ 'cv__rail-item--active': a.id === activeArtifactId }"
          @click="activeArtifactId = a.id"
        >
          <span class="cv__rail-title">{{ a.title }}</span>
          <span class="cv__rail-time">{{ relativeTime(a.updatedAt) }}</span>
        </button>
      </aside>

      <section class="cv__stage workbench-card">
        <div v-if="activeArtifact" class="cv__stage-bar">
          <span class="cv__stage-id">{{ activeArtifact.id }}</span>
          <button
            type="button"
            class="btn-ghost"
            title="Delete artifact"
            @click="removeArtifact(activeArtifact.id)"
          >
            <Trash2 :size="14" />
          </button>
        </div>
        <OmCanvasRenderer v-if="activeArtifact" :doc="activeArtifact" />
        <p v-else class="cv__empty">Select an Auto UI board or create one from Chat.</p>
      </section>
    </div>

    <!-- Freeform Board (Excalidraw) -->
    <div v-else-if="tab === 'board'" class="cv__auto">
      <aside class="cv__rail workbench-card">
        <div class="cv__rail-head">
          <Pencil :size="14" />
          <span>Boards</span>
        </div>
        <button type="button" class="btn-secondary cv__rail-new" @click="newBoard">
          <Plus :size="14" /> New board
        </button>
        <p v-if="boardError" class="cv__empty">{{ boardError }}</p>
        <p v-else-if="!boards.length" class="cv__rail-empty">
          No freeform boards yet. Create one to draw, add text, and pan/zoom.
          Saved under <code>.openmesh/canvases/boards/</code>.
        </p>
        <button
          v-for="b in boards"
          :key="b.id"
          type="button"
          class="cv__rail-item"
          :class="{ 'cv__rail-item--active': b.id === activeBoardId }"
          @click="activeBoardId = b.id; boardDirty = false; boardStatus = null"
        >
          <span class="cv__rail-title">{{ b.title }}</span>
          <span class="cv__rail-time">{{ relativeTime(b.updatedAt) }}</span>
        </button>
      </aside>

      <section class="cv__stage cv__stage--board workbench-card">
        <div v-if="activeBoard" class="cv__stage-bar">
          <div class="cv__board-meta">
            <span class="cv__stage-id">{{ activeBoard.id }}</span>
            <span v-if="boardSaving" class="cv__board-status">Saving…</span>
            <span v-else-if="boardDirty" class="cv__board-status">Unsaved</span>
            <span v-else-if="boardStatus" class="cv__board-status">
              <Save :size="12" /> {{ boardStatus }}
            </span>
          </div>
          <button
            type="button"
            class="btn-ghost"
            title="Delete board"
            @click="removeBoard(activeBoard.id)"
          >
            <Trash2 :size="14" />
          </button>
        </div>
        <BoardEditor
          v-if="activeBoard"
          :key="`${activeBoard.id}-${boardEpoch}`"
          :board-id="activeBoard.id"
          :scene="activeBoard.scene"
          @change="onBoardSceneChange"
        />
        <div v-else class="cv__board-empty">
          <p class="cv__empty">
            Create a board to sketch freely. Agents can add stickies via
            boardAddSticky — not freehand stroke dumps.
          </p>
          <button type="button" class="btn-secondary" @click="newBoard">
            <Plus :size="14" /> New board
          </button>
        </div>
      </section>
    </div>

    <!-- Network graph -->
    <div v-else class="cv__network">
      <div class="cv__network-bar">
        <div>
          <h2 class="cv__network-title">
            {{ canvas.active.value?.title || "Agent network" }}
          </h2>
          <p class="cv__meta">Nodes & edges the agent can control — not screenshots.</p>
        </div>
        <div class="cv__ops">
          <button type="button" class="btn-secondary" @click="addMachine">
            <Plus :size="14" /> Add machine
          </button>
          <button
            type="button"
            class="btn-secondary"
            :disabled="nodes.length < 2"
            @click="connectLastTwo"
          >
            <Link2 :size="14" /> Connect last two
          </button>
          <button type="button" class="btn-secondary" @click="canvas.requestFitView()">
            <Maximize2 :size="14" /> Fit
          </button>
        </div>
      </div>
      <p v-if="canvas.error.value" class="cv__empty">{{ canvas.error.value }}</p>
      <div v-else class="cv__svg-shell workbench-card">
        <svg class="cv__svg" :viewBox="viewBox">
          <defs>
            <marker id="arrow" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto">
              <path d="M0,0 L6,3 L0,6 Z" fill="currentColor" opacity="0.55" />
            </marker>
          </defs>
          <line
            v-for="e in edges"
            :key="e.id"
            :x1="(nodes.find((n) => n.id === e.from)?.x ?? 0) + 70"
            :y1="(nodes.find((n) => n.id === e.from)?.y ?? 0) + 28"
            :x2="nodes.find((n) => n.id === e.to)?.x ?? 0"
            :y2="(nodes.find((n) => n.id === e.to)?.y ?? 0) + 28"
            stroke="currentColor"
            stroke-opacity="0.4"
            stroke-width="2"
            marker-end="url(#arrow)"
          />
          <g v-for="n in nodes" :key="n.id" :transform="`translate(${n.x},${n.y})`">
            <rect width="140" height="56" rx="10" class="cv__node" />
            <text x="70" y="24" text-anchor="middle" class="cv__kind">{{ n.kind }}</text>
            <text x="70" y="42" text-anchor="middle" class="cv__label">{{ n.label }}</text>
          </g>
        </svg>
      </div>
    </div>
  </div>
</template>

<style scoped>
.cv {
  padding: 1.25rem 1.5rem 2rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  min-height: calc(100vh - 96px);
}

.cv__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  flex-wrap: wrap;
}

.cv__title {
  margin: 0;
  font-size: 1.35rem;
  font-weight: 650;
  letter-spacing: -0.02em;
}

.cv__meta {
  margin: 0.25rem 0 0;
  font-size: 0.8125rem;
  color: var(--muted-foreground);
}

.cv__empty {
  margin: 1rem 0;
  color: var(--muted-foreground);
  font-size: 0.875rem;
}

.cv__auto {
  display: grid;
  grid-template-columns: 240px minmax(0, 1fr);
  gap: 0.85rem;
  flex: 1;
  min-height: 0;
}

@media (max-width: 860px) {
  .cv__auto {
    grid-template-columns: 1fr;
  }
}

.cv__rail {
  padding: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  max-height: calc(100vh - 180px);
  overflow: auto;
}

.cv__rail-head {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted-foreground);
  margin-bottom: 0.35rem;
}

.cv__rail-new {
  width: 100%;
  justify-content: center;
  margin-bottom: 0.35rem;
}

.cv__rail-empty {
  margin: 0.5rem 0;
  font-size: 0.8rem;
  color: var(--muted-foreground);
  line-height: 1.45;
}

.cv__rail-empty code {
  font-size: 0.72rem;
}

.cv__rail-item {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.15rem;
  width: 100%;
  text-align: left;
  padding: 0.55rem 0.65rem;
  border-radius: 10px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--foreground);
  cursor: pointer;
}

.cv__rail-item:hover {
  background: var(--surface-highlight);
}

.cv__rail-item--active {
  border-color: var(--border-strong);
  background: var(--surface-2);
}

.cv__rail-title {
  font-size: 0.875rem;
  font-weight: 550;
}

.cv__rail-time {
  font-size: 0.7rem;
  color: var(--muted-foreground);
}

.cv__stage {
  padding: 1.1rem 1.25rem 1.4rem;
  min-height: 360px;
  overflow: auto;
}

.cv__stage--board {
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
}

.cv__stage-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.85rem;
}

.cv__stage--board .cv__stage-bar {
  margin-bottom: 0;
}

.cv__stage-id {
  font-size: 0.7rem;
  font-family: var(--font-mono);
  color: var(--muted-foreground);
}

.cv__board-meta {
  display: flex;
  align-items: center;
  gap: 0.65rem;
}

.cv__board-status {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.7rem;
  color: var(--muted-foreground);
}

.cv__board-empty {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 1rem 0;
}

.cv__network {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  flex: 1;
}

.cv__network-bar {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 1rem;
  flex-wrap: wrap;
}

.cv__network-title {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 600;
}

.cv__ops {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.cv__svg-shell {
  flex: 1;
  min-height: 420px;
  overflow: hidden;
  padding: 0;
}

.cv__svg {
  width: 100%;
  height: min(62vh, 640px);
  color: var(--muted-foreground);
}

.cv__node {
  fill: var(--surface-2);
  stroke: color-mix(in srgb, var(--accent-blue) 45%, var(--border));
  stroke-width: 1.5;
}

.cv__kind {
  fill: var(--muted-foreground);
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.cv__label {
  fill: var(--foreground);
  font-size: 13px;
  font-weight: 600;
}
</style>
