<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useStore } from "../lib/useStore";
import * as fileSystemAdapter from "../lib/adapters/fileSystemAdapter";
import { FolderOpen, Trash2 } from "lucide-vue-next";

const router = useRouter();
const { currentProject, updateProject, deleteProject } = useStore();

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
  if (currentProject.value) {
    form.value = {
      name: currentProject.value.name,
      folderPath: currentProject.value.folderPath,
      repoUrl: currentProject.value.repoUrl || "",
      defaultBranch: currentProject.value.defaultBranch || "main",
      docsFolder: currentProject.value.docsFolder || "",
      terminalDir: currentProject.value.terminalDir || "",
      defaultAgentCli: (currentProject.value.defaultAgentCli as any) || "",
      notes: currentProject.value.notes || "",
      status: currentProject.value.status,
    };
  }
});

const isValid = ref(false);

function validate(): boolean {
  errors.value = {};
  if (!form.value.name.trim()) errors.value.name = "Project name is required";
  if (!form.value.folderPath.trim())
    errors.value.folderPath = "Folder path is required";
  isValid.value = Object.keys(errors.value).length === 0;
  return isValid.value;
}

async function handleChooseFolder() {
  const result = await fileSystemAdapter.pickFolder();
  if (result.success && result.path) {
    form.value.folderPath = result.path;
    validate();
  }
}

async function handleSave() {
  if (!validate() || !currentProject.value) return;

  await updateProject({
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

async function handleDelete() {
  if (!currentProject.value) return;
  if (
    confirm(
      `Delete project "${currentProject.value.name}"?\n\nThis will remove all associated data (docs, sprints, tasks, sessions, presets). Original files on disk are NOT deleted.`,
    )
  ) {
    await deleteProject();
    router.push("/");
  }
}

function handleCancel() {
  router.push("/");
}
</script>

<template>
  <div class="max-w-2xl mx-auto space-y-8 animate-fade-in">
    <div v-if="!currentProject" class="workbench-card p-12 text-center">
      <p class="text-[15px] font-semibold">Project not found</p>
      <button
        @click="router.push('/')"
        class="btn-ghost mt-3 text-[13px] text-muted"
      >
        Go home
      </button>
    </div>

    <template v-else>
      <div>
        <h1 class="text-title">Edit Project</h1>
        <p class="text-body text-muted mt-1">
          Update project settings or delete the project.
        </p>
      </div>

      <form @submit.prevent="handleSave" class="space-y-6">
        <div class="workbench-card p-6 space-y-4">
          <div class="sidebar-section-label !px-0">Project Details</div>

          <div>
            <label class="block text-caption font-medium mb-2 text-muted">Project Name</label>
            <input
              v-model="form.name"
              type="text"
              class="input-luxury w-full"
              :class="{ 'border-red-500': errors.name }"
              @input="validate"
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
                class="input-luxury flex-1"
                :class="{ 'border-red-500': errors.folderPath }"
                @input="validate"
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

          <div>
            <label class="block text-caption font-medium mb-2 text-muted">Repo URL or Path</label>
            <input
              v-model="form.repoUrl"
              type="text"
              class="input-luxury w-full"
            />
          </div>

          <div>
            <label class="block text-caption font-medium mb-2 text-muted">Default Branch</label>
            <input
              v-model="form.defaultBranch"
              type="text"
              class="input-luxury w-full"
            />
          </div>

          <div>
            <label class="block text-caption font-medium mb-2 text-muted">Docs Folder</label>
            <input
              v-model="form.docsFolder"
              type="text"
              class="input-luxury w-full"
            />
          </div>

          <div>
            <label class="block text-caption font-medium mb-2 text-muted">Default Terminal Directory</label>
            <input
              v-model="form.terminalDir"
              type="text"
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
            <label class="block text-caption font-medium mb-2 text-muted">Status</label>
            <select
              v-model="form.status"
              class="input-luxury w-full"
            >
              <option value="active">Active</option>
              <option value="archived">Archived</option>
            </select>
          </div>

          <div>
            <label class="block text-caption font-medium mb-2 text-muted">Notes</label>
            <textarea
              v-model="form.notes"
              rows="3"
              class="input-luxury w-full resize-none"
            ></textarea>
          </div>
        </div>

        <div class="flex gap-3">
          <button
            type="submit"
            :disabled="!isValid"
            class="btn-primary disabled:opacity-30"
          >
            Save Changes
          </button>
          <button type="button" @click="handleCancel" class="btn-secondary">
            Cancel
          </button>
        </div>
      </form>

      <!-- Danger zone -->
      <div
        class="workbench-card p-6 space-y-3"
        style="border-color: rgba(239, 68, 68, 0.2)"
      >
        <h3 class="text-heading" style="color: #ef4444">
          Danger Zone
        </h3>
        <p class="text-caption text-muted">
          Deleting a project removes it from Openmesh along with all associated
          data (docs, sprints, tasks, sessions, presets). Original files on disk
          are NOT deleted.
        </p>
        <button
          @click="handleDelete"
          class="btn-danger flex items-center gap-2"
        >
          <Trash2 class="h-4 w-4" />
          Delete Project
        </button>
      </div>
    </template>
  </div>
</template>
