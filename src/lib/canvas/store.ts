import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export type CanvasNode = {
  id: string;
  label: string;
  kind: string;
  x: number;
  y: number;
};

export type CanvasEdge = {
  id: string;
  from: string;
  to: string;
};

export type CanvasDocument = {
  id: string;
  title: string;
  schemaVersion: string;
  nodes: CanvasNode[];
  edges: CanvasEdge[];
  updatedAt: number;
};

const active = ref<CanvasDocument | null>(null);
const list = ref<CanvasDocument[]>([]);
const fitToken = ref(0);
const error = ref<string | null>(null);

export function useCanvasStore() {
  const nodeCount = computed(() => active.value?.nodes.length ?? 0);

  async function refreshList(projectPath: string) {
    try {
      list.value = await invoke<CanvasDocument[]>("canvas_list", { projectPath });
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    }
  }

  async function ensureCanvas(projectPath: string, title = "Agent network") {
    if (active.value) return active.value;
    try {
      const created = await invoke<CanvasDocument>("canvas_create", {
        projectPath,
        title,
      });
      active.value = created;
      await refreshList(projectPath);
      return created;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      throw e;
    }
  }

  async function load(projectPath: string, id: string) {
    const doc = await invoke<CanvasDocument>("canvas_load", { projectPath, id });
    active.value = doc;
    return doc;
  }

  async function addNode(projectPath: string, label: string, kind?: string) {
    const canvas = await ensureCanvas(projectPath);
    active.value = await invoke<CanvasDocument>("canvas_add_node", {
      projectPath,
      canvasId: canvas.id,
      label,
      kind: kind ?? "machine",
    });
    return `Added ${label}`;
  }

  async function connect(projectPath: string, from: string, to: string) {
    const canvas = active.value ?? (await ensureCanvas(projectPath));
    active.value = await invoke<CanvasDocument>("canvas_connect", {
      projectPath,
      canvasId: canvas.id,
      from,
      to,
    });
    return `Connected ${from} → ${to}`;
  }

  async function deleteNode(projectPath: string, nodeId: string) {
    const canvas = active.value;
    if (!canvas) throw new Error("No canvas open");
    active.value = await invoke<CanvasDocument>("canvas_delete_node", {
      projectPath,
      canvasId: canvas.id,
      nodeId,
    });
    return `Deleted ${nodeId}`;
  }

  function requestFitView() {
    fitToken.value += 1;
  }

  return {
    active,
    list,
    fitToken,
    error,
    nodeCount,
    refreshList,
    ensureCanvas,
    load,
    addNode,
    connect,
    deleteNode,
    requestFitView,
  };
}
