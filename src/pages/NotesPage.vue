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
  importFile,
} = useStore();

const selectedFilename = ref<string | null>(null);
const selectedContent = ref<string>("");
const isEditing = ref(true);
const isDragging = ref(false);
const loading = ref(false);

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
  const filename = "untitled.md";
  selectedFilename.value = filename;
  selectedContent.value = "";
  isEditing.value = true;
}

async function handleDeleteNote() {
  if (!selectedFilename.value) return;
  if (confirm("Delete this note? This cannot be undone.")) {
    await deleteNote(selectedFilename.value);
    selectedFilename.value = null;
    selectedContent.value = "";
  }
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
        <button
          v-for="note in projectNotes"
          :key="note.name"
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
          <div class="text-[13px] truncate">{{ note.name.replace(/\.md$/, "") }}</div>
          <div class="text-[11px] mt-1 text-subtle">
            {{ formatTime(note.modified_at) }}
          </div>
        </button>
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
          <div
            class="flex-1 text-[14px] font-semibold truncate"
            style="color: var(--foreground)"
          >
            {{ selectedFilename.replace(/\.md$/, "") }}
          </div>
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
