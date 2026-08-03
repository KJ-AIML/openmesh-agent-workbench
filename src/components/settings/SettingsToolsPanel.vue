<script setup lang="ts">
import { ref } from "vue";
import { Play, Plus, Terminal, Trash2 } from "lucide-vue-next";
import { useStore } from "../../lib/useStore";
import * as terminalAdapter from "../../lib/adapters/terminalAdapter";

const {
  currentProject,
  projectCommandPresets,
  addCommandPreset,
  deleteCommandPreset,
  addRecentItem,
} = useStore();

const emit = defineEmits<{ toast: [msg: string] }>();

const newPresetName = ref("");
const newPresetCommand = ref("");
const newPresetArgs = ref("");
const newPresetRisk = ref<"safe" | "caution" | "dangerous">("safe");

async function handleOpenTerminal() {
  if (!currentProject.value) return;
  const result = await terminalAdapter.openTerminal({
    workingDir:
      currentProject.value.terminalDir || currentProject.value.folderPath,
  });
  if (result.success) {
    emit("toast", "Terminal opened");
    await addRecentItem({
      type: "terminal",
      title: `Terminal: ${currentProject.value.name}`,
      projectId: currentProject.value.id,
      sourcePath:
        currentProject.value.terminalDir || currentProject.value.folderPath,
    });
  } else if (result.error) {
    emit("toast", result.error);
  }
}

function handleAddPreset() {
  if (
    !newPresetName.value.trim() ||
    !newPresetCommand.value.trim() ||
    !currentProject.value
  ) {
    return;
  }
  const args = newPresetArgs.value.trim().split(/\s+/).filter(Boolean);
  addCommandPreset({
    projectId: currentProject.value.id,
    name: newPresetName.value.trim(),
    command: newPresetCommand.value.trim(),
    args,
    riskLevel: newPresetRisk.value,
    cwd: currentProject.value.terminalDir || currentProject.value.folderPath,
  });
  newPresetName.value = "";
  newPresetCommand.value = "";
  newPresetArgs.value = "";
  newPresetRisk.value = "safe";
  emit("toast", "Preset added");
}

async function handleRunPreset(presetId: string) {
  const preset = projectCommandPresets.value.find((p) => p.id === presetId);
  if (!preset || !currentProject.value) return;
  if (preset.riskLevel === "dangerous") {
    if (
      !confirm(
        `Dangerous command: ${preset.command} ${preset.args.join(" ")}\n\nRun anyway?`,
      )
    ) {
      return;
    }
  } else if (preset.riskLevel === "caution") {
    if (
      !confirm(
        `Caution: ${preset.command} ${preset.args.join(" ")}\n\nContinue?`,
      )
    ) {
      return;
    }
  }
  const result = await terminalAdapter.runCommandPreset(
    preset.command,
    preset.args,
    preset.cwd ||
      currentProject.value.terminalDir ||
      currentProject.value.folderPath,
  );
  if (result.success) {
    emit("toast", `Ran: ${preset.name}`);
    await addRecentItem({
      type: "command_preset",
      title: `Preset: ${preset.name}`,
      projectId: currentProject.value.id,
      sourcePath: preset.cwd || currentProject.value.folderPath,
    });
  } else if (result.error) {
    emit("toast", result.error);
  }
}

function handleDeletePreset(presetId: string) {
  if (!confirm("Delete this command preset?")) return;
  deleteCommandPreset(presetId);
  emit("toast", "Preset deleted");
}
</script>

<template>
  <div class="space-y-4">
    <div v-if="!currentProject" class="text-[13px] text-muted">
      Select a project to manage terminal shortcuts and command presets.
    </div>
    <template v-else>
      <div class="flex flex-wrap items-center gap-2">
        <button type="button" class="btn-primary inline-flex items-center gap-2" @click="handleOpenTerminal">
          <Terminal class="h-4 w-4" />
          Open Terminal
        </button>
        <span class="text-[11px] text-muted font-mono truncate">
          {{ currentProject.terminalDir || currentProject.folderPath }}
        </span>
      </div>

      <div class="space-y-3 pt-2">
        <h4 class="text-[12px] font-semibold text-muted uppercase tracking-wide">
          Command presets
        </h4>
        <div
          v-if="!projectCommandPresets.length"
          class="text-[12px] text-muted"
        >
          No presets yet for this project.
        </div>
        <div
          v-for="p in projectCommandPresets"
          :key="p.id"
          class="flex items-center gap-2 rounded-lg px-3 py-2 text-[12px]"
          style="background: var(--surface-2); border: 1px solid var(--border)"
        >
          <div class="min-w-0 flex-1">
            <div class="font-medium truncate">{{ p.name }}</div>
            <div class="text-muted font-mono truncate text-[11px]">
              {{ p.command }} {{ p.args?.join(" ") }}
            </div>
          </div>
          <span class="badge badge-muted">{{ p.riskLevel }}</span>
          <button type="button" class="btn-ghost p-1.5" title="Run" @click="handleRunPreset(p.id)">
            <Play class="h-3.5 w-3.5" />
          </button>
          <button type="button" class="btn-ghost p-1.5" title="Delete" @click="handleDeletePreset(p.id)">
            <Trash2 class="h-3.5 w-3.5" />
          </button>
        </div>

        <div class="grid gap-2 sm:grid-cols-2">
          <input v-model="newPresetName" class="input-luxury" placeholder="Preset name" />
          <input v-model="newPresetCommand" class="input-luxury" placeholder="Command" />
          <input v-model="newPresetArgs" class="input-luxury sm:col-span-2" placeholder="Args (space-separated)" />
          <select v-model="newPresetRisk" class="input-luxury">
            <option value="safe">safe</option>
            <option value="caution">caution</option>
            <option value="dangerous">dangerous</option>
          </select>
          <button type="button" class="btn-secondary inline-flex items-center justify-center gap-1.5" @click="handleAddPreset">
            <Plus class="h-3.5 w-3.5" />
            Add preset
          </button>
        </div>
      </div>
    </template>
  </div>
</template>
