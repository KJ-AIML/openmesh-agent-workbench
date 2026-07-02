<script setup lang="ts">
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useStore } from "../lib/useStore";
import * as fileSystemAdapter from "../lib/adapters/fileSystemAdapter";

const router = useRouter();
const { addProject } = useStore();

const form = ref({
  name: "",
  folderPath: "",
  repoUrl: "",
  defaultBranch: "main",
  docsFolder: "",
  terminalDir: "",
  defaultAgentCli: "" as "" | "codex" | "claude-code" | "opencode",
  notes: "",
});

const errors = ref<Record<string, string>>({});

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
  if (!validate()) return;

  addProject({
    name: form.value.name.trim(),
    folderPath: form.value.folderPath.trim(),
    repoUrl: form.value.repoUrl.trim() || undefined,
    defaultBranch: form.value.defaultBranch || "main",
    docsFolder: form.value.docsFolder.trim() || undefined,
    terminalDir: form.value.terminalDir.trim() || undefined,
    defaultAgentCli: form.value.defaultAgentCli || null,
    notes: form.value.notes.trim() || undefined,
  });

  router.push("/");
}

function handleCancel() {
  router.push("/");
}
</script>

<template>
  <div class="max-w-2xl mx-auto space-y-6">
    <div>
      <h1 class="text-2xl font-bold">Add Project</h1>
      <p class="text-sm mt-1" style="color: var(--muted-foreground)">
        Add a project to Openmesh. All work context will be anchored to this project.
      </p>
    </div>

    <form @submit.prevent="handleSave" class="space-y-4">
      <!-- Required fields -->
      <div class="space-y-4 rounded-lg border p-4" style="border-color: var(--border)">
        <div class="text-xs font-medium uppercase tracking-wider" style="color: var(--muted-foreground)">
          Required
        </div>

        <div>
          <label class="block text-sm font-medium mb-1">Project Name</label>
          <input
            v-model="form.name"
            type="text"
            placeholder="e.g., OpenMesh"
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
              placeholder="e.g., C:\KJ\Repos\open-mesh-lab"
              class="flex-1 rounded-md border px-3 py-2 text-sm"
              style="background: var(--background); border-color: var(--border); color: var(--foreground)"
              :class="{ 'border-red-500': errors.folderPath }"
            />
            <button
              type="button"
              @click="handleChooseFolder"
              class="rounded-md border px-3 py-2 text-sm font-medium transition-colors whitespace-nowrap"
              style="border-color: var(--border); color: var(--foreground); background: var(--background)"
            >
              Choose Folder
            </button>
          </div>
          <p v-if="errors.folderPath" class="text-xs text-red-500 mt-1">{{ errors.folderPath }}</p>
        </div>
      </div>

      <!-- Optional fields -->
      <div class="space-y-4 rounded-lg border p-4" style="border-color: var(--border)">
        <div class="text-xs font-medium uppercase tracking-wider" style="color: var(--muted-foreground)">
          Optional
        </div>

        <div>
          <label class="block text-sm font-medium mb-1">Repo URL or Path</label>
          <input
            v-model="form.repoUrl"
            type="text"
            placeholder="https://github.com/... or local path"
            class="w-full rounded-md border px-3 py-2 text-sm"
            style="background: var(--background); border-color: var(--border); color: var(--foreground)"
          />
        </div>

        <div>
          <label class="block text-sm font-medium mb-1">Default Branch</label>
          <input
            v-model="form.defaultBranch"
            type="text"
            placeholder="main"
            class="w-full rounded-md border px-3 py-2 text-sm"
            style="background: var(--background); border-color: var(--border); color: var(--foreground)"
          />
        </div>

        <div>
          <label class="block text-sm font-medium mb-1">Docs Folder</label>
          <input
            v-model="form.docsFolder"
            type="text"
            placeholder="docs/"
            class="w-full rounded-md border px-3 py-2 text-sm"
            style="background: var(--background); border-color: var(--border); color: var(--foreground)"
          />
        </div>

        <div>
          <label class="block text-sm font-medium mb-1">Default Terminal Directory</label>
          <input
            v-model="form.terminalDir"
            type="text"
            placeholder="Defaults to folder path"
            class="w-full rounded-md border px-3 py-2 text-sm"
            style="background: var(--background); border-color: var(--border); color: var(--foreground)"
          />
        </div>

        <div>
          <label class="block text-sm font-medium mb-1">Default Agent CLI</label>
          <select
            v-model="form.defaultAgentCli"
            class="w-full rounded-md border px-3 py-2 text-sm"
            style="background: var(--background); border-color: var(--border); color: var(--foreground)"
          >
            <option value="">None</option>
            <option value="codex">Codex</option>
            <option value="claude-code">Claude Code</option>
            <option value="opencode">OpenCode</option>
          </select>
        </div>

        <div>
          <label class="block text-sm font-medium mb-1">Notes</label>
          <textarea
            v-model="form.notes"
            rows="3"
            placeholder="Any notes about this project..."
            class="w-full rounded-md border px-3 py-2 text-sm resize-none"
            style="background: var(--background); border-color: var(--border); color: var(--foreground)"
          ></textarea>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex gap-3">
        <button
          type="submit"
          :disabled="!isValid"
          class="rounded-md px-4 py-2 text-sm font-medium transition-colors"
          style="background: var(--foreground); color: var(--background)"
          :class="{ 'opacity-50 cursor-not-allowed': !isValid }"
        >
          Save Project
        </button>
        <button
          type="button"
          @click="handleCancel"
          class="rounded-md px-4 py-2 text-sm font-medium transition-colors"
          style="border: 1px solid var(--border); color: var(--muted-foreground)"
        >
          Cancel
        </button>
      </div>
    </form>
  </div>
</template>
