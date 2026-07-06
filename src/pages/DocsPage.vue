<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useRoute } from "vue-router";
import { marked } from "marked";
import { useStore } from "../lib/useStore";
import type { DocTreeNode } from "../lib/store";
import DocTreeItem from "../components/DocTreeItem.vue";
import {
  Plus,
  FileText,
  Trash2,
  Edit,
  Eye,
  FolderOpen,
  FolderPlus,
  ChevronRight,
  Upload,
  Check,
  X,
  Pencil,
} from "lucide-vue-next";

const route = useRoute();

const {
  currentProject,
  projectDocs,
  docsTree,
  refreshDocs,
  deleteDoc,
  readDoc,
  writeDoc,
  importFile,
  createDocFolder,
  renameDocFolder,
  deleteDocFolder,
  renameDoc,
  moveDoc,
} = useStore();

// --- State ---
const selectedPath = ref<string | null>(null);
const selectedContent = ref<string>("");
const isEditing = ref(false);
const isDragging = ref(false);
const loading = ref(false);
const error = ref<string | null>(null);

// Folder creation
const showNewFolder = ref(false);
const newFolderName = ref("");

// Inline rename
const renamingPath = ref<string | null>(null);
const renameValue = ref("");

// Expanded folders
const expandedFolders = ref<Set<string>>(new Set(["root"]));

// Context menu
const contextMenu = ref<{ x: number; y: number; node: DocTreeNode } | null>(null);

// Drag and drop
const dragOverPath = ref<string | null>(null);
const pointerDrag = ref<{
  node: DocTreeNode;
  startX: number;
  startY: number;
  x: number;
  y: number;
  active: boolean;
} | null>(null);
const suppressSelectPath = ref<string | null>(null);

// Inline confirm dialog
const confirmDialog = ref<{ message: string; onConfirm: () => void } | null>(null);

// Toast notifications
const toast = ref<{ message: string; type: "error" | "success" } | null>(null);
let toastTimeout: ReturnType<typeof setTimeout> | null = null;

function showToast(message: string, type: "error" | "success" = "error") {
  toast.value = { message, type };
  if (toastTimeout) clearTimeout(toastTimeout);
  toastTimeout = setTimeout(() => { toast.value = null; }, 3000);
}

function showConfirm(message: string, onConfirm: () => void) {
  confirmDialog.value = { message, onConfirm };
}

// --- Computed ---
const selectedDoc = computed(() => {
  if (!selectedPath.value) return null;
  return findNodeByPath(docsTree.value, selectedPath.value);
});

const isFolderSelected = computed(() => {
  return selectedDoc.value?.nodeType === "folder";
});

const renderedContent = computed(() => marked(selectedContent.value || ""));

const breadcrumbs = computed(() => {
  if (!selectedPath.value) return [];
  const parts = selectedPath.value.split("/");
  const crumbs: { label: string; path: string }[] = [];
  let acc = "";
  for (const part of parts) {
    acc = acc ? `${acc}/${part}` : part;
    crumbs.push({ label: part, path: acc });
  }
  return crumbs;
});

// --- Tree helpers ---
function findNodeByPath(nodes: DocTreeNode[], path: string): DocTreeNode | null {
  for (const node of nodes) {
    if (node.path === path) return node;
    if (node.children) {
      const found = findNodeByPath(node.children, path);
      if (found) return found;
    }
  }
  return null;
}

function toggleFolder(path: string) {
  const next = new Set(expandedFolders.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  expandedFolders.value = next;
}

function parentPath(path: string): string {
  const index = path.lastIndexOf("/");
  return index === -1 ? "" : path.slice(0, index);
}

function joinPath(parent: string, name: string): string {
  return parent ? `${parent}/${name}` : name;
}

// --- Actions ---
async function handleSelectDoc(node: DocTreeNode) {
  if (suppressSelectPath.value === node.path) return;
  if (node.nodeType === "folder") {
    toggleFolder(node.path);
    return;
  }
  selectedPath.value = node.path;
  loading.value = true;
  error.value = null;
  try {
    selectedContent.value = await readDoc(node.path);
    isEditing.value = false;
  } catch (e) {
    error.value = e instanceof Error ? e.message : "Failed to read doc";
    selectedContent.value = "";
  } finally {
    loading.value = false;
  }
}

async function handleNewDoc() {
  if (!currentProject.value) return;
  const existingNames = projectDocs.value.map((d) => d.name.split("/").pop() || "");
  let counter = 1;
  let filename = `untitled-${counter}.md`;
  while (existingNames.includes(filename)) {
    counter++;
    filename = `untitled-${counter}.md`;
  }
  selectedPath.value = filename;
  selectedContent.value = "";
  isEditing.value = true;
  await writeDoc(filename, "");
  await refreshDocs();
}

async function handleDeleteDoc() {
  if (!selectedPath.value) return;
  showConfirm(`Delete "${selectedPath.value}"? This cannot be undone.`, async () => {
    await deleteDoc(selectedPath.value!);
    selectedPath.value = null;
    selectedContent.value = "";
    confirmDialog.value = null;
  });
}

async function handleContentChange(newContent: string) {
  selectedContent.value = newContent;
  if (!selectedPath.value) return;
  clearTimeout((window as any).__docSaveTimeout);
  (window as any).__docSaveTimeout = setTimeout(async () => {
    try {
      await writeDoc(selectedPath.value!, newContent);
    } catch (e) {
      console.error("Failed to save doc:", e);
    }
  }, 500);
}

async function handleCreateFolder() {
  const name = newFolderName.value.trim();
  if (!name) return;
  try {
    await createDocFolder(name);
    newFolderName.value = "";
    showNewFolder.value = false;
    expandedFolders.value = new Set([...expandedFolders.value, "root"]);
  } catch (e) {
    showToast(e instanceof Error ? e.message : "Failed to create folder");
  }
}

function startRename(node: DocTreeNode) {
  renamingPath.value = node.path;
  renameValue.value = node.name;
  contextMenu.value = null;
}

async function handleRename() {
  if (!renamingPath.value) return;
  const newName = renameValue.value.trim();
  if (!newName) { renamingPath.value = null; return; }
  const current = findNodeByPath(docsTree.value, renamingPath.value);
  if (!current) { renamingPath.value = null; return; }
  if (newName === current.name) { renamingPath.value = null; return; }

  try {
    if (current.nodeType === "folder") {
      const newPath = joinPath(parentPath(renamingPath.value), newName);
      await renameDocFolder(renamingPath.value, newPath);
      if (selectedPath.value === renamingPath.value || selectedPath.value?.startsWith(`${renamingPath.value}/`)) {
        selectedPath.value = selectedPath.value.replace(renamingPath.value, newPath);
      }
    } else {
      // For files, ensure .md extension
      const finalName = newName.endsWith('.md') ? newName : `${newName}.md`;
      const finalPath = joinPath(parentPath(renamingPath.value), finalName);
      await renameDoc(renamingPath.value, finalPath);
      // Update selected path if we renamed the currently selected file
      if (selectedPath.value === renamingPath.value) {
        selectedPath.value = finalPath;
      }
    }
    renamingPath.value = null;
  } catch (e) {
    showToast(e instanceof Error ? e.message : "Failed to rename");
    renamingPath.value = null;
  }
}

async function handleDeleteFolder(node: DocTreeNode) {
  showConfirm(`Delete folder "${node.name}" and all its contents?`, async () => {
    try {
      await deleteDocFolder(node.path);
      if (selectedPath.value?.startsWith(node.path + "/") || selectedPath.value === node.path) {
        selectedPath.value = null;
        selectedContent.value = "";
      }
    } catch (e) {
      showToast(e instanceof Error ? e.message : "Failed to delete folder");
    }
    confirmDialog.value = null;
  });
  contextMenu.value = null;
}

async function handleDeleteFile(node: DocTreeNode) {
  showConfirm(`Delete "${node.name}"? This cannot be undone.`, async () => {
    try {
      await deleteDoc(node.path);
      if (selectedPath.value === node.path) {
        selectedPath.value = null;
        selectedContent.value = "";
      }
    } catch (e) {
      showToast(e instanceof Error ? e.message : "Failed to delete file");
    }
    confirmDialog.value = null;
  });
  contextMenu.value = null;
}

function folderNodeFromPoint(x: number, y: number): DocTreeNode | null {
  const element = document.elementFromPoint(x, y);
  const folderElement = element?.closest("[data-doc-folder]") as HTMLElement | null;
  const folderPath = folderElement?.dataset.docFolder;
  return folderPath ? findNodeByPath(docsTree.value, folderPath) : null;
}

async function moveNodeToFolder(sourceNode: DocTreeNode, targetNode: DocTreeNode) {
  if (targetNode.nodeType !== "folder") return;

  const sourcePath = sourceNode.path;
  const targetFolder = targetNode.path;

  if (parentPath(sourcePath) === targetFolder) {
    return;
  }

  try {
    await moveDoc(sourcePath, targetFolder);
    if (selectedPath.value === sourcePath) {
      selectedPath.value = joinPath(targetFolder, sourceNode.name);
    }
    showToast(`Moved "${sourceNode.name}" to "${targetNode.name}"`, "success");
  } catch (e) {
    showToast(e instanceof Error ? e.message : "Failed to move file");
  }
}

function cleanupPointerDrag() {
  window.removeEventListener("pointermove", handlePointerMove);
  window.removeEventListener("pointerup", handlePointerUp);
}

function handlePointerDragStart(event: PointerEvent, node: DocTreeNode) {
  pointerDrag.value = {
    node,
    startX: event.clientX,
    startY: event.clientY,
    x: event.clientX,
    y: event.clientY,
    active: false,
  };
  window.addEventListener("pointermove", handlePointerMove);
  window.addEventListener("pointerup", handlePointerUp);
}

function handlePointerMove(event: PointerEvent) {
  const drag = pointerDrag.value;
  if (!drag) return;

  const dx = Math.abs(event.clientX - drag.startX);
  const dy = Math.abs(event.clientY - drag.startY);
  if (!drag.active && dx + dy < 6) return;

  event.preventDefault();
  drag.active = true;
  drag.x = event.clientX;
  drag.y = event.clientY;

  const targetNode = folderNodeFromPoint(event.clientX, event.clientY);
  dragOverPath.value = targetNode?.path ?? null;
}

async function handlePointerUp(event: PointerEvent) {
  const drag = pointerDrag.value;
  cleanupPointerDrag();
  pointerDrag.value = null;

  if (!drag?.active) {
    dragOverPath.value = null;
    return;
  }

  event.preventDefault();
  suppressSelectPath.value = drag.node.path;
  setTimeout(() => {
    if (suppressSelectPath.value === drag.node.path) suppressSelectPath.value = null;
  }, 0);

  const targetNode = folderNodeFromPoint(event.clientX, event.clientY);
  dragOverPath.value = null;
  if (targetNode) {
    await moveNodeToFolder(drag.node, targetNode);
  }
}

function hasExternalFiles(e: DragEvent): boolean {
  return Array.from(e.dataTransfer?.items || []).some((item) => item.kind === "file");
}

function handlePageDragOver(e: DragEvent) {
  if (!hasExternalFiles(e)) return;
  e.preventDefault();
  isDragging.value = true;
}
function handlePageDragLeave() { isDragging.value = false; }

async function handlePageDrop(e: DragEvent) {
  e.preventDefault();
  isDragging.value = false;
  if (!currentProject.value) return;
  const files = Array.from(e.dataTransfer?.files || []);
  if (files.length === 0) return;
  for (const file of files.filter((f) => f.name.endsWith(".md"))) {
    const content = await new Promise<string>((resolve) => {
      const reader = new FileReader();
      reader.onload = (ev) => resolve(ev.target?.result as string);
      reader.readAsText(file);
    });
    await importFile("docs", file.name, content);
  }
  await refreshDocs();
}

function formatBytes(bytes: number | null): string {
  if (!bytes) return "";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

// Wait for the docs tree to be populated, with a bounded retry so
// the deep-link works even if refreshDocs() takes longer than
// expected (e.g., slow Rust IPC on desktop).
async function ensureTreeLoaded(): Promise<void> {
  if (docsTree.value.length > 0) return;
  let attempts = 0;
  while (docsTree.value.length === 0 && attempts < 20) {
    await refreshDocs();
    if (docsTree.value.length === 0) {
      await new Promise((r) => setTimeout(r, 50));
    }
    attempts++;
  }
  if (docsTree.value.length === 0) {
    await refreshDocs();
  }
}

// Handle a deep-link from Context Search: normalize the path, find the
// exact node, expand all parent folders, and select + load the file.
// Shared between onMounted (initial load) and the route query watcher
// (subsequent navigations while DocsPage is already mounted).
async function handleDeepLink(fileParam: string): Promise<void> {
  // Normalize the incoming path: decode URL encoding and convert
  // any Windows-style backslashes to POSIX forward slashes so it
  // matches the tree node paths returned by the Rust backend.
  const normalizedParam = normalizeTreePath(fileParam);
  await ensureTreeLoaded();
  // Find the node in the tree using normalized path.
  const node = findNodeByPath(docsTree.value, normalizedParam);
  if (node && node.nodeType === "file") {
    // Expand all parent folders using normalized path segments.
    const parts = normalizedParam.split("/").filter(Boolean);
    for (let i = 1; i < parts.length; i++) {
      const folderPath = parts.slice(0, i).join("/");
      expandedFolders.value.add(folderPath);
    }
    // Select the file and load its content.
    await handleSelectDoc(node);
  }
}

onMounted(async () => {
  if (currentProject.value) {
    await ensureTreeLoaded();
    // Handle deep-link from Context Search
    const fileParam = route.query.file;
    if (typeof fileParam === "string" && fileParam) {
      await handleDeepLink(fileParam);
    }
  }
});

// Handle deep-link when the route query changes while DocsPage is
// already mounted (component reuse). Without this, opening a second
// doc from Context Search without first navigating away from /docs
// would not expand folders or select the file.
watch(
  () => route.query.file,
  async (fileParam) => {
    if (typeof fileParam === "string" && fileParam) {
      await handleDeepLink(fileParam);
    }
  },
);

/**
 * Normalize a path from a route query parameter to match the format
 * used by Rust's `list_docs_tree_fn` (POSIX forward slashes, no URL
 * encoding, relative to docs root).
 */
function normalizeTreePath(raw: string): string {
  // Decode any URL encoding (e.g., %20 for spaces, %2F for slashes).
  let decoded: string;
  try {
    decoded = decodeURIComponent(raw);
  } catch {
    decoded = raw;
  }
  // Replace backslashes with forward slashes for Windows compatibility.
  return decoded.replace(/\\/g, "/");
}
watch(() => currentProject.value, async () => {
  if (currentProject.value) {
    await refreshDocs();
    selectedPath.value = null;
    selectedContent.value = "";
  }
});
</script>

<template>
  <div
    class="flex h-[calc(100vh-56px)] animate-fade-in"
    @dragover="handlePageDragOver"
    @dragleave="handlePageDragLeave"
    @drop="handlePageDrop"
    @click="contextMenu = null"
  >
    <!-- Sidebar: Tree view -->
    <div
      class="w-72 flex-shrink-0 flex flex-col"
      style="border-right: 1px solid var(--divider); background: var(--surface-1)"
    >
      <div class="flex items-center justify-between px-5 py-4" style="border-bottom: 1px solid var(--divider)">
        <h2 class="text-heading">Docs</h2>
        <div class="flex gap-1">
          <button @click="showNewFolder = !showNewFolder" :disabled="!currentProject" class="btn-ghost p-2 disabled:opacity-30" title="New folder">
            <FolderPlus class="h-4 w-4" />
          </button>
          <button @click="handleNewDoc" :disabled="!currentProject" class="btn-ghost p-2 disabled:opacity-30" title="New doc">
            <Plus class="h-4 w-4" />
          </button>
        </div>
      </div>

      <div v-if="showNewFolder" class="px-4 py-2 flex items-center gap-2" style="border-bottom: 1px solid var(--divider); background: var(--surface-2)">
        <input v-model="newFolderName" @keyup.enter="handleCreateFolder" @keyup.escape="showNewFolder = false"
          class="flex-1 bg-transparent border border-[var(--border)] rounded-lg px-2 py-1 text-[12px] outline-none"
          style="color: var(--foreground)" placeholder="Folder name" autofocus />
        <button @click="handleCreateFolder" class="btn-ghost p-1" style="color: var(--accent-green)"><Check class="h-3.5 w-3.5" /></button>
        <button @click="showNewFolder = false" class="btn-ghost p-1"><X class="h-3.5 w-3.5" /></button>
      </div>

      <div v-if="!currentProject" class="p-5 text-[12px] text-muted">Select a project to view docs</div>
      <div v-else-if="docsTree.length === 0" class="p-5 text-[12px] text-muted">No docs yet. Create one or drag .md files here.</div>

      <div v-else class="flex-1 overflow-y-auto py-2">
        <DocTreeItem
          v-for="node in docsTree"
          :key="node.path"
          :node="node"
          :depth="0"
          :selected-path="selectedPath"
          :expanded-folders="expandedFolders"
          :renaming-path="renamingPath"
          :rename-value="renameValue"
          :drag-over-path="dragOverPath"
          @select="handleSelectDoc"
          @rename-start="startRename"
          @rename-commit="handleRename"
          @rename-cancel="renamingPath = null"
          @rename-update="renameValue = $event"
          @delete-folder="handleDeleteFolder"
          @delete-file="handleDeleteFile"
          @context-menu="contextMenu = $event"
          @pointer-drag-start="handlePointerDragStart"
        />
      </div>
    </div>

    <!-- Content area -->
    <div class="flex-1 flex flex-col min-w-0">
      <div v-if="!selectedPath" class="flex-1 flex items-center justify-center" style="color: var(--muted-foreground)">
        <div v-if="isDragging" class="text-center">
          <div class="flex h-16 w-16 mx-auto mb-4 items-center justify-center rounded-2xl" style="background: var(--surface-2); border: 1px solid var(--border)">
            <Upload class="h-7 w-7" />
          </div>
          <div class="text-[15px] font-semibold mb-1">Drop .md files here</div>
          <div class="text-sm text-muted">to import as docs</div>
        </div>
        <div v-else-if="!currentProject" class="text-center">
          <div class="flex h-16 w-16 mx-auto mb-4 items-center justify-center rounded-2xl" style="background: var(--surface-2); border: 1px solid var(--border)">
            <FolderOpen class="h-7 w-7" />
          </div>
          <div class="text-[15px] font-semibold mb-1">No project selected</div>
          <div class="text-sm text-muted">Select a project to view docs</div>
        </div>
        <div v-else class="text-center">
          <div class="flex h-16 w-16 mx-auto mb-4 items-center justify-center rounded-2xl" style="background: var(--surface-2); border: 1px solid var(--border)">
            <FileText class="h-7 w-7 text-subtle" />
          </div>
          <div class="text-[15px] font-semibold mb-1">No doc selected</div>
          <div class="text-sm text-muted">Select a doc from the sidebar</div>
        </div>
      </div>

      <div v-else class="flex-1 flex flex-col min-w-0">
        <!-- Breadcrumbs -->
        <div class="flex items-center gap-1 px-5 py-2 flex-shrink-0 text-[11px]" style="border-bottom: 1px solid var(--divider); color: var(--muted-foreground)">
          <span class="cursor-pointer hover:underline" @click="selectedPath = null">docs</span>
          <template v-for="(crumb, i) in breadcrumbs" :key="i">
            <ChevronRight class="h-3 w-3" />
            <span v-if="i < breadcrumbs.length - 1" class="cursor-pointer hover:underline">{{ crumb.label }}</span>
            <span v-else class="font-medium" style="color: var(--foreground)">{{ crumb.label }}</span>
          </template>
        </div>

        <!-- Toolbar -->
        <div class="flex items-center gap-2 px-5 py-3 flex-shrink-0" style="border-bottom: 1px solid var(--divider)">
          <div class="flex-1 text-[14px] font-semibold truncate" style="color: var(--foreground)">
            {{ selectedDoc?.name || selectedPath }}
          </div>
          <div v-if="selectedDoc?.size" class="text-[10px] text-subtle mr-2">{{ formatBytes(selectedDoc.size) }}</div>
          <button @click="startRename(selectedDoc!)" class="btn-ghost flex items-center gap-1.5" title="Rename">
            <Pencil class="h-4 w-4" />
            <span class="text-[12px]">Rename</span>
          </button>
          <button @click="isEditing = !isEditing" class="btn-ghost flex items-center gap-1.5" :title="isEditing ? 'Preview' : 'Edit'">
            <Eye v-if="isEditing" class="h-4 w-4" />
            <Edit v-else class="h-4 w-4" />
            <span class="text-[12px]">{{ isEditing ? "Preview" : "Edit" }}</span>
          </button>
          <button @click="handleDeleteDoc" class="btn-ghost" style="color: #ef4444" title="Delete doc">
            <Trash2 class="h-4 w-4" />
          </button>
        </div>

        <div v-if="error" class="px-5 py-3 text-sm" style="color: #ef4444; background: rgba(239,68,68,0.05); border-bottom: 1px solid var(--divider)">{{ error }}</div>

        <div class="flex-1 overflow-auto">
          <div v-if="isFolderSelected" class="p-6 text-center text-muted">
            <div class="flex h-16 w-16 mx-auto mb-4 items-center justify-center rounded-2xl" style="background: var(--surface-2); border: 1px solid var(--border)">
              <FolderOpen class="h-7 w-7" />
            </div>
            <div class="text-[15px] font-semibold mb-1" style="color: var(--foreground)">{{ selectedDoc?.name }}</div>
            <div class="text-sm text-muted">
              {{ selectedDoc?.children?.length ? `${selectedDoc.children.length} item(s) in this folder` : "Empty folder" }}
            </div>
          </div>
          <div v-else-if="loading" class="p-6 text-sm text-muted">Loading...</div>
          <textarea v-else-if="isEditing" :value="selectedContent"
            @input="handleContentChange(($event.target as HTMLTextAreaElement).value)"
            class="w-full h-full p-6 bg-transparent border-none outline-none resize-none text-[13px] leading-relaxed"
            style="color: var(--foreground); font-family: var(--font-mono)" placeholder="Start writing in markdown..." />
          <div v-else class="p-6 prose prose-invert max-w-none" v-html="renderedContent" />
        </div>
      </div>
    </div>

    <!-- Context menu -->
    <div v-if="contextMenu" class="fixed z-50 rounded-xl py-1 shadow-lg"
      style="background: var(--surface-2); border: 1px solid var(--border); min-width: 160px"
      :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }">
      <button v-if="contextMenu.node.nodeType === 'file'" @click="startRename(contextMenu.node); contextMenu = null"
        class="w-full text-left px-3 py-2 text-[12px] hover:bg-[var(--surface-highlight)] flex items-center gap-2"
        style="color: var(--foreground)">
        <Edit class="h-3.5 w-3.5" /> Rename
      </button>
      <button v-if="contextMenu.node.nodeType === 'folder'" @click="handleDeleteFolder(contextMenu.node)"
        class="w-full text-left px-3 py-2 text-[12px] hover:bg-[var(--surface-highlight)] flex items-center gap-2"
        style="color: #ef4444">
        <Trash2 class="h-3.5 w-3.5" /> Delete folder
      </button>
      <button v-if="contextMenu.node.nodeType === 'file'" @click="handleDeleteFile(contextMenu.node)"
        class="w-full text-left px-3 py-2 text-[12px] hover:bg-[var(--surface-highlight)] flex items-center gap-2"
        style="color: #ef4444">
        <Trash2 class="h-3.5 w-3.5" /> Delete file
      </button>
    </div>

    <!-- Confirm dialog -->
    <div v-if="confirmDialog" class="fixed inset-0 z-50 flex items-center justify-center"
      style="background: rgba(0,0,0,0.6)" @click.self="confirmDialog = null">
      <div class="rounded-2xl p-6 max-w-sm w-full mx-4"
        style="background: var(--surface-2); border: 1px solid var(--border); box-shadow: 0 20px 60px rgba(0,0,0,0.5)">
        <p class="text-[14px] mb-5" style="color: var(--foreground)">{{ confirmDialog.message }}</p>
        <div class="flex gap-2 justify-end">
          <button @click="confirmDialog = null" class="btn-ghost text-[12px]">Cancel</button>
          <button @click="confirmDialog?.onConfirm()" class="btn-primary text-[12px]" style="background: #ef4444; color: white">Delete</button>
        </div>
      </div>
    </div>

    <!-- Toast notification -->
    <div v-if="toast" class="fixed bottom-6 right-6 z-50 rounded-xl px-4 py-3 shadow-lg text-[13px]"
      :style="{
        background: toast.type === 'error' ? 'rgba(239,68,68,0.15)' : 'rgba(16,185,129,0.15)',
        border: `1px solid ${toast.type === 'error' ? 'rgba(239,68,68,0.3)' : 'rgba(16,185,129,0.3)'}`,
        color: toast.type === 'error' ? '#fca5a5' : '#6ee7b7'
      }">
      {{ toast.message }}
    </div>

    <div
      v-if="pointerDrag?.active"
      class="fixed z-[80] pointer-events-none rounded-lg px-3 py-2 text-[12px] shadow-xl"
      :style="{
        left: `${pointerDrag.x + 12}px`,
        top: `${pointerDrag.y + 12}px`,
        background: 'var(--surface-2)',
        border: '1px solid var(--border)',
        color: 'var(--foreground)'
      }"
    >
      {{ pointerDrag.node.name }}
    </div>
  </div>
</template>

<style scoped>
.prose { color: var(--foreground); }
.prose :deep(h1), .prose :deep(h2), .prose :deep(h3), .prose :deep(h4) {
  color: var(--foreground); margin-top: 1.5em; margin-bottom: 0.5em; letter-spacing: -0.02em;
}
.prose :deep(p) { margin-bottom: 1em; line-height: 1.7; }
.prose :deep(code) { background: var(--surface-1); padding: 0.2em 0.4em; border-radius: 4px; font-size: 0.85em; font-family: var(--font-mono); }
.prose :deep(pre) { background: var(--surface-1); padding: 1em; border-radius: 8px; overflow-x: auto; border: 1px solid var(--border); }
.prose :deep(pre code) { background: transparent; padding: 0; }
.prose :deep(a) { color: #60a5fa; text-decoration: underline; }
.prose :deep(blockquote) { border-left: 2px solid var(--border); padding-left: 1em; margin-left: 0; color: var(--muted-foreground); }
.prose :deep(ul), .prose :deep(ol) { margin-left: 1.5em; margin-bottom: 1em; }
.prose :deep(li) { margin-bottom: 0.25em; }
.prose :deep(img) { max-width: 100%; border-radius: 8px; margin: 1em 0; }
.prose :deep(table) { width: 100%; border-collapse: collapse; margin: 1em 0; }
.prose :deep(th), .prose :deep(td) { border: 1px solid var(--border); padding: 0.5em; text-align: left; }
.prose :deep(th) { background: var(--surface-1); font-weight: 600; }
</style>
