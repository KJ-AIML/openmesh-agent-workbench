<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onUnmounted } from "vue";
import {
  Folder,
  Terminal,
  GitBranch,
  Zap,
  Bot,
  Code2,
  Scan,
  Play,
  FileText,
  FileEdit,
  ListTodo,
  Settings,
  Plus,
  Search,
  ArrowRight,
  Lock,
} from "lucide-vue-next";
import type { Command, CommandGroup } from "../lib/commands";
import { GROUP_ORDER } from "../lib/commands";

const props = defineProps<{
  commands: Command[];
  visible: boolean;
}>();

const emit = defineEmits<{
  close: [];
  executed: [command: Command];
}>();

const search = ref("");
const selectedIndex = ref(0);
const searchInputRef = ref<HTMLInputElement | null>(null);

const iconMap: Record<string, any> = {
  folder: Folder,
  terminal: Terminal,
  git: GitBranch,
  zap: Zap,
  bot: Bot,
  code: Code2,
  scan: Scan,
  play: Play,
  "file-text": FileText,
  "file-edit": FileEdit,
  "list-todo": ListTodo,
  settings: Settings,
  plus: Plus,
};

// Filter commands by search query
const filteredCommands = computed(() => {
  const query = search.value.toLowerCase().trim();
  if (!query) return props.commands;

  return props.commands.filter((cmd) => {
    const titleMatch = cmd.title.toLowerCase().includes(query);
    const descMatch = cmd.description.toLowerCase().includes(query);
    const groupMatch = cmd.group.toLowerCase().includes(query);
    return titleMatch || descMatch || groupMatch;
  });
});

// Group commands for display
const groupedCommands = computed(() => {
  const groups: { group: CommandGroup; commands: Command[] }[] = [];
  for (const groupName of GROUP_ORDER) {
    const cmds = filteredCommands.value.filter((c) => c.group === groupName);
    if (cmds.length > 0) {
      groups.push({ group: groupName, commands: cmds });
    }
  }
  return groups;
});

// Flat list for keyboard navigation (only available commands)
const availableCommands = computed(() =>
  filteredCommands.value.filter((c) => c.available),
);

// Reset selection when search changes
watch(search, () => {
  selectedIndex.value = 0;
});

// Reset when palette opens
watch(
  () => props.visible,
  async (isVisible) => {
    if (isVisible) {
      search.value = "";
      selectedIndex.value = 0;
      await nextTick();
      searchInputRef.value?.focus();
    }
  },
);

function handleKeydown(e: KeyboardEvent) {
  if (!props.visible) return;

  if (e.key === "Escape") {
    e.preventDefault();
    emit("close");
    return;
  }

  if (e.key === "ArrowDown") {
    e.preventDefault();
    const maxIndex = availableCommands.value.length - 1;
    selectedIndex.value = Math.min(selectedIndex.value + 1, maxIndex);
    return;
  }

  if (e.key === "ArrowUp") {
    e.preventDefault();
    selectedIndex.value = Math.max(selectedIndex.value - 1, 0);
    return;
  }

  if (e.key === "Enter") {
    e.preventDefault();
    const cmd = availableCommands.value[selectedIndex.value];
    if (cmd) {
      emit("executed", cmd);
      emit("close");
    }
    return;
  }
}

function handleCommandClick(cmd: Command) {
  if (!cmd.available) return;
  emit("executed", cmd);
  emit("close");
}

function getIcon(iconName: string) {
  return iconMap[iconName] || FileText;
}

onMounted(() => {
  document.addEventListener("keydown", handleKeydown);
});

onUnmounted(() => {
  document.removeEventListener("keydown", handleKeydown);
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="command-palette-overlay"
      @click.self="emit('close')"
    >
      <div class="command-palette">
        <!-- Search input -->
        <div class="command-palette-search">
          <Search class="command-palette-search-icon" />
          <input
            ref="searchInputRef"
            v-model="search"
            type="text"
            placeholder="Type a command or search..."
            class="command-palette-input"
            @keydown.stop
          />
          <kbd class="command-palette-kbd">ESC</kbd>
        </div>

        <!-- Command list -->
        <div class="command-palette-list">
          <div
            v-if="filteredCommands.length === 0"
            class="command-palette-empty"
          >
            <Search class="h-5 w-5" style="color: var(--muted-foreground); opacity: 0.5" />
            <p class="text-[13px]" style="color: var(--muted-foreground)">
              No commands found
            </p>
          </div>

          <template v-else>
            <div
              v-for="group in groupedCommands"
              :key="group.group"
              class="command-palette-group"
            >
              <div class="command-palette-group-label">
                {{ group.group }}
              </div>
              <button
                v-for="(cmd, idx) in group.commands"
                :key="cmd.id"
                class="command-palette-item"
                :class="{
                  'command-palette-item-active':
                    cmd.available &&
                    availableCommands.indexOf(cmd) === selectedIndex,
                  'command-palette-item-disabled': !cmd.available,
                }"
                :disabled="!cmd.available"
                @click="handleCommandClick(cmd)"
                @mouseenter="
                  if (cmd.available) {
                    selectedIndex = availableCommands.indexOf(cmd);
                  }
                "
              >
                <component
                  :is="getIcon(cmd.icon)"
                  class="command-palette-item-icon"
                  :class="{
                    'command-palette-item-icon-disabled': !cmd.available,
                  }"
                />
                <div class="command-palette-item-content">
                  <span class="command-palette-item-title">
                    {{ cmd.title }}
                  </span>
                  <span
                    v-if="!cmd.available && cmd.disabledReason"
                    class="command-palette-item-reason"
                  >
                    <Lock class="h-3 w-3" />
                    {{ cmd.disabledReason }}
                  </span>
                  <span v-else class="command-palette-item-desc">
                    {{ cmd.description }}
                  </span>
                </div>
                <ArrowRight
                  v-if="cmd.available"
                  class="command-palette-item-arrow"
                />
              </button>
            </div>
          </template>
        </div>

        <!-- Footer hint -->
        <div class="command-palette-footer">
          <span>
            <kbd>↑↓</kbd> navigate
          </span>
          <span>
            <kbd>↵</kbd> run
          </span>
          <span>
            <kbd>esc</kbd> close
          </span>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.command-palette-overlay {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 15vh;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  animation: fadeIn 0.12s ease-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@keyframes slideUp {
  from {
    opacity: 0;
    transform: translateY(-8px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

.command-palette {
  width: 560px;
  max-width: 90vw;
  max-height: 70vh;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 16px;
  box-shadow:
    0 8px 32px rgba(0, 0, 0, 0.5),
    0 2px 8px rgba(0, 0, 0, 0.3);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  animation: slideUp 0.15s ease-out;
}

.command-palette-search {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid var(--border);
  background: var(--surface-1);
}

.command-palette-search-icon {
  height: 1rem;
  width: 1rem;
  color: var(--muted-foreground);
  opacity: 0.6;
  flex-shrink: 0;
}

.command-palette-input {
  flex: 1;
  background: transparent;
  border: none;
  outline: none;
  color: var(--foreground);
  font-size: 0.875rem;
  font-family: var(--font-sans);
  letter-spacing: -0.01em;
}

.command-palette-input::placeholder {
  color: var(--muted-foreground);
  opacity: 0.5;
}

.command-palette-kbd {
  font-size: 0.625rem;
  font-weight: 600;
  padding: 0.15rem 0.4rem;
  border-radius: 4px;
  background: var(--surface-2);
  color: var(--muted-foreground);
  border: 1px solid var(--border);
  flex-shrink: 0;
}

.command-palette-list {
  overflow-y: auto;
  padding: 0.375rem;
  flex: 1;
}

.command-palette-group {
  margin-bottom: 0.25rem;
}

.command-palette-group-label {
  font-size: 0.625rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--muted-foreground);
  opacity: 0.6;
  padding: 0.375rem 0.625rem 0.25rem;
}

.command-palette-item {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  width: 100%;
  padding: 0.5rem 0.625rem;
  border-radius: 10px;
  border: none;
  background: transparent;
  color: var(--foreground);
  cursor: pointer;
  transition: all 0.1s ease;
  text-align: left;
  font-family: var(--font-sans);
}

.command-palette-item:hover:not(:disabled) {
  background: var(--surface-highlight);
}

.command-palette-item-active {
  background: var(--surface-3) !important;
}

.command-palette-item-disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.command-palette-item-icon {
  height: 1rem;
  width: 1rem;
  color: var(--muted-foreground);
  flex-shrink: 0;
  opacity: 0.7;
}

.command-palette-item-icon-disabled {
  opacity: 0.4;
}

.command-palette-item-content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.command-palette-item-title {
  font-size: 0.8125rem;
  font-weight: 500;
  color: var(--foreground);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.command-palette-item-desc {
  font-size: 0.6875rem;
  color: var(--muted-foreground);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.command-palette-item-reason {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.6875rem;
  color: var(--accent-amber);
}

.command-palette-item-arrow {
  height: 0.875rem;
  width: 0.875rem;
  color: var(--muted-foreground);
  opacity: 0.4;
  flex-shrink: 0;
}

.command-palette-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  padding: 2rem 1rem;
}

.command-palette-footer {
  display: flex;
  gap: 1rem;
  padding: 0.5rem 1rem;
  border-top: 1px solid var(--border);
  background: var(--surface-1);
  font-size: 0.6875rem;
  color: var(--muted-foreground);
}

.command-palette-footer kbd {
  font-size: 0.625rem;
  font-weight: 600;
  padding: 0.1rem 0.3rem;
  border-radius: 3px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  margin-right: 0.25rem;
}
</style>
