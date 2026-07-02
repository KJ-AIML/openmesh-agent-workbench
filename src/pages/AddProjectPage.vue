<script setup lang="ts">
import { ref, computed } from "vue";
import { useRouter } from "vue-router";
import { useStore } from "../lib/useStore";
import * as fileSystemAdapter from "../lib/adapters/fileSystemAdapter";
import { FolderOpen } from "lucide-vue-next";

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
  if (!form.value.folderPath.trim())
    errors.value.folderPath = "Folder path is required";
  return Object.keys(errors.value).length === 0;
}

async function handleChooseFolder() {
  const result = await fileSystemAdapter.pickFolder();
  if (result.success && result.path) {
    form.value.folderPath = result.path;
  }
}

async function handleSave() {
  if (!validate()) return;

  try {
    await addProject({
      name: form.value.name.trim(),
      folderPath: form.value.folderPath.trim(),
      repoUrl: form.value.repoUrl.trim() || undefined,
      defaultBranch: form.value.defaultBranch || "main",
      docsFolder: form.value.docsFolder.trim() || undefined,
      terminalDir: form.value.terminalDir.trim() || undefined,
      defaultAgentCli: form.value.defaultAgentCli || null,
      notes: form.value.notes.trim() || undefined,
    });

    await new Promise((resolve) => setTimeout(resolve, 100));
    router.push("/");
  } catch (e) {
    console.error("Failed to add project:", e);
    alert("Failed to create project. Please check the folder path and try again.");
  }
}

function handleCancel() {
  router.push("/");
}
</script>

<template>
  <div class="max-w-2xl mx-auto space-y-8 animate-fade-in">
    <div>
      <h1 class="text-title">Add Project</h1>
      <p class="text-body text-muted mt-1">
        Add a project to Openmesh. All work context will be anchored to this project.
      </p>
    </div>

    <form @submit.prevent="handleSave" class="space-y-6">
      <!-- Required fields -->
      <div class="workbench-card p-6 space-y-4">
        <div class="sidebar-section-label !px-0">Required</div>

        <div>
          <label class="block text-caption font-medium mb-2 text-muted">Project Name</label>
          <input
            v-model="form.name"
            type="text"
            placeholder="e.g., OpenMesh"
            class="input-luxury w-full"
            :class="{ 'border-red-500': errors.name }"
          />
          <p v-if="errors.name" class="text-[12px] mt-2" style="color: #ef4444">
            {{ errors.name }}
          </p>
        </div>

        <div>
          <label class="block text-caption font-medium mb-2 text-muted">Local Folder Path</label>
          <div class="flex gap-2">
            <input
              v-model="form.folderPath"
              type="text"
              placeholder="e.g., C:\KJ\Repos\open-mesh-lab"
              class="input-luxury flex-1"
              :class="{ 'border-red-500': errors.folderPath }"
            />
            <button
              type="button"
              @click="handleChooseFolder"
              class="btn-secondary flex items-center gap-1.5"
            >
              <FolderOpen class="h-4 w-4" />
              Choose Folder
            </button>
          </div>
          <p v-if="errors.folderPath" class="text-[12px] mt-2" style="color: #ef4444">
            {{ errors.folderPath }}
          </p>
        </div>
      </div>

      <!-- Optional fields -->
      <div class="workbench-card p-6 space-y-4">
        <div class="sidebar-section-label !px-0">Optional</div>

        <div>
          <label class="block text-caption font-medium mb-2 text-muted">Repo URL or Path</label>
          <input
            v-model="form.repoUrl"
            type="text"
            placeholder="https://github.com/... or local path"
            class="input-luxury w-full"
          />
        </div>

        <div>
          <label class="block text-caption font-medium mb-2 text-muted">Default Branch</label>
          <input
            v-model="form.defaultBranch"
            type="text"
            placeholder="main"
            class="input-luxury w-full"
          />
        </div>

        <div>
          <label class="block text-caption font-medium mb-2 text-muted">Docs Folder</label>
          <input
            v-model="form.docsFolder"
            type="text"
            placeholder="docs/"
            class="input-luxury w-full"
          />
        </div>

        <div>
          <label class="block text-caption font-medium mb-2 text-muted">Default Terminal Directory</label>
          <input
            v-model="form.terminalDir"
            type="text"
            placeholder="Defaults to folder path"
            class="input-luxury w-full"
          />
        </div>

        <div>
          <label class="block text-caption font-medium mb-2 text-muted">Default Agent CLI</label>
          <select
            v-model="form.defaultAgentCli"
            class="input-luxury w-full"
          >
            <option value="">None</option>
            <option value="codex">Codex</option>
            <option value="claude-code">Claude Code</option>
            <option value="opencode">OpenCode</option>
          </select>
        </div>

        <div>
          <label class="block text-caption font-medium mb-2 text-muted">Notes</label>
          <textarea
            v-model="form.notes"
            rows="3"
            placeholder="Any notes about this project..."
            class="input-luxury w-full resize-none"
          ></textarea>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex gap-3">
        <button
          type="submit"
          :disabled="!isValid"
          class="btn-primary disabled:opacity-30"
        >
          Save Project
        </button>
        <button
          type="button"
          @click="handleCancel"
          class="btn-secondary"
        >
          Cancel
        </button>
      </div>
    </form>
  </div>
</template>
