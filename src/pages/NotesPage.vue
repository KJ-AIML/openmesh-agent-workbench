<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { marked } from "marked";
import { useStore } from "../lib/useStore";
import { Plus, Trash2, Edit, Eye, FileEdit, FolderOpen } from "lucide-vue-next";

const {
  currentProject,
  projectNotes,
  refreshNotes,
  readNote,
  writeNote,
  deleteNote,
  renameNote,
  importFile,
} = useStore();

const selectedFilename = ref<string | null>(null);
const selectedContent = ref<string>("");
const isEditing = ref(true);
const isDragging = ref(false);
const loading = ref(false);
const isRenaming = ref(false);
const renameValue = ref("");

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

const selectedNote = computed(() =>
  projectNotes.value.find((n) => n.name === selectedFilename.value),
);

const renderedContent = computed(() => {
  return marked(selectedContent.value || "");
});

onMounted(async () => {
  if (currentProject.value) {
    await refreshNotes();
  }
});

watch(
  () => currentProject.value,
  async () => {
    if (currentProject.value) {
      await refreshNotes();
      selectedFilename.value = null;
      selectedContent.value = "";
    }
  },
);

async function handleSelectNote(filename: string) {
  selectedFilename.value = filename;
  loading.value = true;
  try {
    selectedContent.value = await readNote(filename);
  } catch (e) {
    console.error("Failed to read note:", e);
    selectedContent.value = "";
  } finally {
    loading.value = false;
  }
}

async function handleNewNote() {
  if (!currentProject.value) return;

  // Generate unique filename
  const existingNames = projectNotes.value.map(n => n.name);
  let counter = 1;
  let filename = `untitled-${counter}.md`;
  while (existingNames.includes(filename)) {
    counter++;
    filename = `untitled-${counter}.md`;
  }

  selectedFilename.value = filename;
  selectedContent.value = "";
  isEditing.value = true;

  // Create the file immediately so it appears in the list
  await writeNote(filename, "");
  await refreshNotes();
}

async function handleRenameNote() {
  if (!selectedFilename.value || !currentProject.value) return;

  const newFilename = renameValue.value.trim();
  if (!newFilename || newFilename === selectedFilename.value) {
    isRenaming.value = false;
    return;
  }

  // Ensure .md extension
  const finalName = newFilename.endsWith('.md') ? newFilename : `${newFilename}.md`;

  // Check if filename already exists
  const exists = projectNotes.value.some(n => n.name === finalName);
  if (exists) {
    showToast(`A note named "${finalName}" already exists.`);
    return;
  }

  try {
    clearTimeout((window as any).__noteSaveTimeout);
    await writeNote(selectedFilename.value, selectedContent.value);
    await renameNote(selectedFilename.value, finalName);
    selectedFilename.value = finalName;
    isRenaming.value = false;
  } catch (e) {
    showToast(e instanceof Error ? e.message : "Failed to rename note");
  }
}

function startRename() {
  if (!selectedFilename.value) return;
  renameValue.value = selectedFilename.value.replace(/\.md$/, "");
  isRenaming.value = true;
}

async function handleDeleteNote() {
  if (!selectedFilename.value) return;
  showConfirm("Delete this note? This cannot be undone.", async () => {
    await deleteNote(selectedFilename.value!);
    selectedFilename.value = null;
    selectedContent.value = "";
    confirmDialog.value = null;
  });
}

async function handleContentChange(newContent: string) {
  selectedContent.value = newContent;
  if (!selectedFilename.value || !currentProject.value) return;

  clearTimeout((window as any).__noteSaveTimeout);
  (window as any).__noteSaveTimeout = setTimeout(async () => {
    try {
      await writeNote(selectedFilename.value!, newContent);
      await refreshNotes();
    } catch (e) {
      console.error("Failed to save note:", e);
    }
  }, 500);
}

function handleDragOver(e: DragEvent) {
  if (!Array.from(e.dataTransfer?.items || []).some((item) => item.kind === "file")) return;
  e.preventDefault();
  isDragging.value = true;
}

function handleDragLeave() {
  isDragging.value = false;
}

async function handleDrop(e: DragEvent) {
  e.preventDefault();
  isDragging.value = false;

  if (!currentProject.value) return;

  const files = Array.from(e.dataTransfer?.files || []);
  const mdFiles = files.filter((f) => f.name.endsWith(".md"));

  for (const file of mdFiles) {
    const reader = new FileReader();
    const content = await new Promise<string>((resolve) => {
      reader.onload = (e) => resolve(e.target?.result as string);
      reader.readAsText(file);
    });

    await importFile("notes", file.name, content);
  }

  await refreshNotes();
}

function formatTime(dateStr: string | null) {
  if (!dateStr) return "";
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString();
}
</script>

<template>
  <div
    class="flex h-[calc(100vh-56px)] animate-fade-in"
    @dragover="handleDragOver"
    @dragleave="handleDragLeave"
    @drop="handleDrop"
  >
    <!-- Notes list -->
    <div
      class="w-72 flex-shrink-0 flex flex-col"
      style="border-right: 1px solid var(--divider); background: var(--surface-1)"
    >
      <div
        class="flex items-center justify-between px-5 py-4"
        style="border-bottom: 1px solid var(--divider)"
      >
        <h2 class="text-heading">Notes</h2>
        <button
          @click="handleNewNote"
          :disabled="!currentProject"
          class="btn-ghost p-2 disabled:opacity-30 disabled:cursor-not-allowed"
          title="New note"
        >
          <Plus class="h-4 w-4" />
        </button>
      </div>

      <div v-if="!currentProject" class="p-5 text-[12px] text-muted">
        Select a project to view notes
      </div>

      <div
        v-else-if="projectNotes.length === 0"
        class="p-5 text-[12px] text-muted"
      >
        No notes yet. Create one or drag .md files here.
      </div>

      <div v-else class="flex-1 overflow-y-auto py-2">
        <div
          v-for="note in projectNotes"
          :key="note.name"
          class="group relative"
        >
          <button
            @click="handleSelectNote(note.name)"
            class="w-full text-left px-4 py-3 transition-all"
            :class="
              selectedFilename === note.name
                ? 'font-medium'
                : 'font-normal hover:bg-[var(--surface-highlight)]'
            "
            :style="
              selectedFilename === note.name
                ? 'background: var(--sidebar-accent); color: var(--foreground)'
                : 'color: var(--muted-foreground)'
            "
          >
            <div class="text-[13px] truncate pr-6">{{ note.name.replace(/\.md$/, "") }}</div>
            <div class="text-[11px] mt-1 text-subtle">
              {{ formatTime(note.modified_at) }}
            </div>
          </button>
          <div
            v-if="selectedFilename === note.name"
            class="absolute right-2 top-1/2 -translate-y-1/2 flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity"
          >
            <button
              @click.stop="selectedFilename = note.name; startRename()"
              class="btn-ghost p-1"
              title="Rename"
            >
              <Edit class="h-3 w-3" />
            </button>
            <button
              @click.stop="selectedFilename = note.name; handleDeleteNote()"
              class="btn-ghost p-1"
              style="color: #ef4444"
              title="Delete"
            >
              <Trash2 class="h-3 w-3" />
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Editor/Preview -->
    <div class="flex-1 flex flex-col min-w-0">
      <div
        v-if="!selectedFilename"
        class="flex-1 flex items-center justify-center"
        style="color: var(--muted-foreground)"
      >
        <div v-if="isDragging" class="text-center">
          <div
            class="flex h-16 w-16 mx-auto mb-4 items-center justify-center rounded-2xl"
            style="background: var(--surface-2); border: 1px solid var(--border)"
          >
            <FileEdit class="h-7 w-7" />
          </div>
          <div class="text-[15px] font-semibold mb-1">Drop .md files here</div>
          <div class="text-sm text-muted">to import as notes</div>
        </div>
        <div v-else-if="!currentProject" class="text-center">
          <div
            class="flex h-16 w-16 mx-auto mb-4 items-center justify-center rounded-2xl"
            style="background: var(--surface-2); border: 1px solid var(--border)"
          >
            <FolderOpen class="h-7 w-7" />
          </div>
          <div class="text-[15px] font-semibold mb-1">No project selected</div>
          <div class="text-sm text-muted">Select a project to view notes</div>
        </div>
        <div v-else class="text-center">
          <div
            class="flex h-16 w-16 mx-auto mb-4 items-center justify-center rounded-2xl"
            style="background: var(--surface-2); border: 1px solid var(--border)"
          >
            <FileEdit class="h-7 w-7 text-subtle" />
          </div>
          <div class="text-[15px] font-semibold mb-1">No note selected</div>
          <div class="text-sm text-muted">Select a note or create a new one</div>
        </div>
      </div>

      <div v-else class="flex-1 flex flex-col min-w-0">
        <!-- Toolbar -->
        <div
          class="flex items-center gap-2 px-5 py-3 flex-shrink-0"
          style="border-bottom: 1px solid var(--divider)"
        >
          <div v-if="isRenaming" class="flex-1 flex items-center gap-2">
            <input
              v-model="renameValue"
              @keyup.enter="handleRenameNote"
              @keyup.escape="isRenaming = false"
              class="flex-1 bg-transparent border-b border-[var(--border)] outline-none text-[14px] font-semibold px-1 py-0.5"
              style="color: var(--foreground)"
              placeholder="Note name"
              autofocus
            />
            <button
              @click="handleRenameNote"
              class="btn-ghost text-[11px]"
              style="color: var(--accent-green)"
            >
              Save
            </button>
            <button
              @click="isRenaming = false"
              class="btn-ghost text-[11px]"
            >
              Cancel
            </button>
          </div>
          <div v-else class="flex-1 text-[14px] font-semibold truncate" style="color: var(--foreground)">
            {{ selectedFilename.replace(/\.md$/, "") }}
          </div>
          <button
            v-if="!isRenaming"
            @click="startRename"
            class="btn-ghost flex items-center gap-1.5"
            title="Rename note"
          >
            <Edit class="h-4 w-4" />
            <span class="text-[12px]">Rename</span>
          </button>
          <button
            @click="isEditing = !isEditing"
            class="btn-ghost flex items-center gap-1.5"
            :title="isEditing ? 'Preview' : 'Edit'"
          >
            <Eye v-if="isEditing" class="h-4 w-4" />
            <Edit v-else class="h-4 w-4" />
            <span class="text-[12px]">{{ isEditing ? "Preview" : "Edit" }}</span>
          </button>
          <button
            @click="handleDeleteNote"
            class="btn-ghost"
            style="color: #ef4444"
            title="Delete note"
          >
            <Trash2 class="h-4 w-4" />
          </button>
        </div>

        <!-- Content -->
        <div class="flex-1 overflow-auto">
          <div
            v-if="loading"
            class="p-6 text-sm text-muted"
          >
            Loading...
          </div>
          <textarea
            v-else-if="isEditing"
            :value="selectedContent"
            @input="
              handleContentChange(($event.target as HTMLTextAreaElement).value)
            "
            class="w-full h-full p-6 bg-transparent border-none outline-none resize-none text-[13px] leading-relaxed"
            style="color: var(--foreground); font-family: var(--font-mono)"
            placeholder="Start writing in markdown..."
          />
          <div
            v-else
            class="p-6 prose prose-invert max-w-none"
            v-html="renderedContent"
          />
        </div>
      </div>
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
  </div>
</template>

<style scoped>
.prose {
  color: var(--foreground);
}

.prose :deep(h1),
.prose :deep(h2),
.prose :deep(h3),
.prose :deep(h4) {
  color: var(--foreground);
  margin-top: 1.5em;
  margin-bottom: 0.5em;
  letter-spacing: -0.02em;
}

.prose :deep(p) {
  margin-bottom: 1em;
  line-height: 1.7;
}

.prose :deep(code) {
  background: var(--surface-1);
  padding: 0.2em 0.4em;
  border-radius: 4px;
  font-size: 0.85em;
  font-family: var(--font-mono);
}

.prose :deep(pre) {
  background: var(--surface-1);
  padding: 1em;
  border-radius: 8px;
  overflow-x: auto;
  border: 1px solid var(--border);
}

.prose :deep(pre code) {
  background: transparent;
  padding: 0;
}

.prose :deep(a) {
  color: #60a5fa;
  text-decoration: underline;
}

.prose :deep(blockquote) {
  border-left: 2px solid var(--border);
  padding-left: 1em;
  margin-left: 0;
  color: var(--muted-foreground);
}

.prose :deep(ul),
.prose :deep(ol) {
  margin-left: 1.5em;
  margin-bottom: 1em;
}

.prose :deep(li) {
  margin-bottom: 0.25em;
}
</style>
