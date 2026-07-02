<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { useRouter } from "vue-router";
import { useStore } from "../lib/useStore";
import { Plus, FileText, Trash2, ExternalLink, FolderOpen } from "lucide-vue-next";

const router = useRouter();
const { currentProject, projectDocs, refreshDocs, deleteDoc, readDoc } = useStore();

const loading = ref(false);
const error = ref<string | null>(null);

onMounted(async () => {
  if (currentProject.value) {
    await refreshDocs();
  }
});

watch(
  () => currentProject.value,
  async () => {
    if (currentProject.value) {
      await refreshDocs();
    }
  },
);

async function handleDelete(filename: string) {
  if (!confirm(`Delete "${filename}"? This cannot be undone.`)) return;

  loading.value = true;
  error.value = null;

  try {
    await deleteDoc(filename);
  } catch (e) {
    error.value = e instanceof Error ? e.message : "Failed to delete doc";
  } finally {
    loading.value = false;
  }
}

async function handleOpen(filename: string) {
  try {
    const content = await readDoc(filename);
    const blob = new Blob([content], { type: "text/markdown" });
    const url = URL.createObjectURL(blob);
    window.open(url, "_blank");
  } catch (e) {
    error.value = e instanceof Error ? e.message : "Failed to open doc";
  }
}

function formatBytes(bytes: number | null): string {
  if (bytes === null || bytes === undefined) return "—";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatDate(dateStr: string | null): string {
  if (!dateStr) return "—";
  const date = new Date(dateStr);
  return (
    date.toLocaleDateString() +
    " " +
    date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
  );
}
</script>

<template>
  <div class="space-y-8 animate-fade-in">
    <div>
      <h1 class="text-title">Docs</h1>
      <p class="text-body text-muted mt-1">
        Project documentation stored in
        <code
          class="rounded-lg px-2 py-1 text-[11px]"
          style="background: var(--surface-1); color: var(--foreground); border: 1px solid var(--border)"
          >.openmesh/docs/</code
        >
      </p>
    </div>

    <div v-if="!currentProject" class="workbench-card p-12 text-center">
      <div
        class="flex h-16 w-16 mx-auto mb-4 items-center justify-center rounded-2xl"
        style="background: var(--surface-2); border: 1px solid var(--border)"
      >
        <FolderOpen class="h-7 w-7 text-subtle" />
      </div>
      <p class="text-[15px] font-semibold">No project selected</p>
      <p class="text-sm mt-1 text-muted">
        Add a project to see docs.
      </p>
    </div>

    <div v-else-if="loading" class="workbench-card p-12 text-center">
      <p class="text-sm text-muted">Loading docs...</p>
    </div>

    <div
      v-else-if="error"
      class="workbench-card p-5"
      style="border-color: rgba(239, 68, 68, 0.3); background: rgba(239, 68, 68, 0.05)"
    >
      <p class="text-sm" style="color: #ef4444">{{ error }}</p>
    </div>

    <div
      v-else-if="projectDocs.length === 0"
      class="workbench-card p-12 text-center"
    >
      <div
        class="flex h-16 w-16 mx-auto mb-4 items-center justify-center rounded-2xl"
        style="background: var(--surface-2); border: 1px solid var(--border)"
      >
        <FileText class="h-7 w-7 text-subtle" />
      </div>
      <p class="text-[15px] font-semibold">No docs yet</p>
      <p class="text-sm mt-1 text-muted">
        Markdown files in <code class="rounded-lg px-2 py-1 text-[11px]" style="background: var(--surface-1)">.openmesh/docs/</code> will appear here.
      </p>
    </div>

    <div v-else class="grid gap-5 md:grid-cols-2 lg:grid-cols-3">
      <div
        v-for="doc in projectDocs"
        :key="doc.name"
        class="workbench-card-compact p-5 space-y-3 group"
      >
        <div class="flex items-start justify-between">
          <div class="flex items-start gap-3">
            <div
              class="flex h-10 w-10 items-center justify-center rounded-xl flex-shrink-0"
              style="background: var(--surface-1); border: 1px solid var(--border)"
            >
              <FileText class="h-5 w-5 text-muted" />
            </div>
            <div>
              <h3 class="text-[13px] font-semibold truncate max-w-[180px]">{{ doc.name }}</h3>
              <p class="text-[11px] mt-0.5 text-muted">
                {{ formatBytes(doc.size) }}
              </p>
            </div>
          </div>
        </div>

        <div class="text-[11px] text-subtle">
          Modified: {{ formatDate(doc.modified_at) }}
        </div>

        <div class="flex gap-2 flex-wrap pt-1">
          <button
            @click="handleOpen(doc.name)"
            class="btn-secondary flex items-center gap-1.5 text-[12px]"
          >
            <ExternalLink class="h-3.5 w-3.5" />
            Open
          </button>
          <button
            @click="handleDelete(doc.name)"
            class="btn-ghost text-[12px]"
            style="color: #ef4444"
          >
            <Trash2 class="h-3.5 w-3.5" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
