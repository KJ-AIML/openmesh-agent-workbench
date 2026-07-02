<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter, useRoute } from "vue-router";
import { useStore } from "../lib/useStore";
import * as fileSystemAdapter from "../lib/adapters/fileSystemAdapter";

const router = useRouter();
const route = useRoute();
const { projects, updateProject, deleteProject, selectProject } = useStore();

const projectId = route.params.id as string;
const project = computed(() => projects.value.find((p) => p.id === projectId));

const form = ref({
  name: "",
  folderPath: "",
  repoUrl: "",
  defaultBranch: "main",
  docsFolder: "",
  terminalDir: "",
  defaultAgentCli: "" as "" | "codex" | "claude-code" | "opencode",
  notes: "",
  status: "active" as "active" | "archived",
});

const errors = ref<Record<string, string>>({});

onMounted(() => {
  if (project.value) {
    form.value = {
      name: project.value.name,
      folderPath: project.value.folderPath,
      repoUrl: project.value.repoUrl || "",
      defaultBranch: project.value.defaultBranch || "main",
      docsFolder: project.value.docsFolder || "",
      terminalDir: project.value.terminalDir || "",
      defaultAgentCli: project.value.defaultAgentCli || "",
      notes: project.value.notes || "",
      status: project.value.status,
    };
  }
});

const isValid = computed(() => {
  return form.value.name.trim() !== "" && form.value.folderPath.trim() !== "";
});

function validate(): boolean {
  errors.value = {};
  if (!form.value.name.trim()) errors.value.name = "Project name is required";
  if (!form.value.folderPath.trim()) errors.value.folderPath = "Folder path is required";
  return Object.keys(errors.value).length === 0;
}

async function handleChooseFolder() {
  const result = await fileSystemAdapter.pickFolder();
  if (result.success && result.path) {
    form.value.folderPath = result.path;
  }
}

function handleSave() {
  if (!validate() || !project.value) return;

  updateProject(project.value.id, {
    name: form.value.name.trim(),
    folderPath: form.value.folderPath.trim(),
    repoUrl: form.value.repoUrl.trim() || undefined,
    defaultBranch: form.value.defaultBranch || "main",
    docsFolder: form.value.docsFolder.trim() || undefined,
    terminalDir: form.value.terminalDir.trim() || undefined,
    defaultAgentCli: form.value.defaultAgentCli || null,
    notes: form.value.notes.trim() || undefined,
    status: form.value.status,
  });

  router.push("/");
}

function handleDelete() {
  if (!project.value) return;
  if (confirm(`Delete project "${project.value.name}"?\n\nThis will remove all associated data (docs, sprints, tasks, sessions, presets). Original files on disk are NOT deleted.`)) {
    deleteProject(project.value.id);
    router.push("/");
  }
}

function handleCancel() {
  router.push("/");
}
</script>

<template>
  <div class="max-w-2xl mx-auto space-y-6">
    <div v-if="!project" class="rounded-lg border p-8 text-center" style="border-color: var(--border)">
      <p class="text-lg font-medium">Project not found</p>
      <button @click="router.push('/')" class="text-sm mt-2 underline" style="color: var(--muted-foreground)">Go home</button>
    </div>

    <template v-else>
      <div>
        <h1 class="text-2xl font-bold">Edit Project</h1>
        <p class="text-sm mt-1" style="color: var(--muted-foreground)">
          Update project settings or delete the project.
        </p>
      </div>

      <form @submit.prevent="handleSave" class="space-y-4">
        <div class="space-y-4 rounded-lg border p-4" style="border-color: var(--border)">
          <div class="text-xs font-medium uppercase tracking-wider" style="color: var(--muted-foreground)">
            Project Details
          </div>

          <div>
            <label class="block text-sm font-medium mb-1">Project Name</label>
            <input
              v-model="form.name"
              type="text"
              class="w-full rounded-md border px-3 py-2 text-sm"
              style="background: var(--background); border-color: var(--border); color: var(--foreground)"
              :class="{ 'border-red-500': errors.name }"
            />
            <p v-if="errors.name" class="text-xs text-red-500 mt-1">{{ errors.name }}</p>
          </div>

          <div>
            <label class="block text-sm font-medium mb-1">Local Folder Path</label>
            <div class="flex gap-2">
              <input
                v-model="form.folderPath"
                type="text"
                class="flex-1 rounded-md border px-3 py-2 text-sm"
                style="background: var(--background); border-color: var(--border); color: var(--foreground)"
                :class="{ 'border-red-500': errors.folderPath }"
              />
              <button
                type="button"
                @click="handleChooseFolder"
                class="rounded-md border px-3 py-2 text-sm font-medium whitespace-nowrap"
                style="border-color: var(--border); color: var(--foreground); background: var(--background)"
              >
                Choose Folder
              </button>
            </div>
            <p v-if="errors.folderPath" class="text-xs text-red-500 mt-1">{{ errors.folderPath }}</p>
          </div>

          <div>
            <label class="block text-sm font-medium mb-1">Repo URL or Path</label>
            <input v-model="form.repoUrl" type="text" class="w-full rounded-md border px-3 py-2 text-sm" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
          </div>

          <div>
            <label class="block text-sm font-medium mb-1">Default Branch</label>
            <input v-model="form.defaultBranch" type="text" class="w-full rounded-md border px-3 py-2 text-sm" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
          </div>

          <div>
            <label class="block text-sm font-medium mb-1">Docs Folder</label>
            <input v-model="form.docsFolder" type="text" class="w-full rounded-md border px-3 py-2 text-sm" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
          </div>

          <div>
            <label class="block text-sm font-medium mb-1">Default Terminal Directory</label>
            <input v-model="form.terminalDir" type="text" class="w-full rounded-md border px-3 py-2 text-sm" style="background: var(--background); border-color: var(--border); color: var(--foreground)" />
          </div>

          <div>
            <label class="block text-sm font-medium mb-1">Default Agent CLI</label>
            <select v-model="form.defaultAgentCli" class="w-full rounded-md border px-3 py-2 text-sm" style="background: var(--background); border-color: var(--border); color: var(--foreground)">
              <option value="">None</option>
              <option value="codex">Codex</option>
              <option value="claude-code">Claude Code</option>
              <option value="opencode">OpenCode</option>
            </select>
          </div>

          <div>
            <label class="block text-sm font-medium mb-1">Status</label>
            <select v-model="form.status" class="w-full rounded-md border px-3 py-2 text-sm" style="background: var(--background); border-color: var(--border); color: var(--foreground)">
              <option value="active">Active</option>
              <option value="archived">Archived</option>
            </select>
          </div>

          <div>
            <label class="block text-sm font-medium mb-1">Notes</label>
            <textarea v-model="form.notes" rows="3" class="w-full rounded-md border px-3 py-2 text-sm resize-none" style="background: var(--background); border-color: var(--border); color: var(--foreground)"></textarea>
          </div>
        </div>

        <div class="flex gap-3">
          <button
            type="submit"
            :disabled="!isValid"
            class="rounded-md px-4 py-2 text-sm font-medium"
            style="background: var(--foreground); color: var(--background)"
            :class="{ 'opacity-50 cursor-not-allowed': !isValid }"
          >
            Save Changes
          </button>
          <button
            type="button"
            @click="handleCancel"
            class="rounded-md px-4 py-2 text-sm font-medium"
            style="border: 1px solid var(--border); color: var(--muted-foreground)"
          >
            Cancel
          </button>
        </div>
      </form>

      <!-- Danger zone -->
      <div class="rounded-lg border p-4 space-y-3" style="border-color: #ef444440">
        <h2 class="text-sm font-semibold" style="color: #ef4444">Danger Zone</h2>
        <p class="text-xs" style="color: var(--muted-foreground)">
          Deleting a project removes it from Openmesh along with all associated data (docs, sprints, tasks, sessions, presets). Original files on disk are NOT deleted.
        </p>
        <button
          @click="handleDelete"
          class="rounded-md px-4 py-2 text-sm font-medium"
          style="border: 1px solid #ef4444; color: #ef4444"
        >
          Delete Project
        </button>
      </div>
    </template>
  </div>
</template>
