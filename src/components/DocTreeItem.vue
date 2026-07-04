<script setup lang="ts">
import { computed } from "vue";
import type { DocTreeNode } from "../lib/store";
import { ChevronRight, ChevronDown, Folder, FileText, MoreVertical, Pencil } from "lucide-vue-next";

const props = defineProps<{
  node: DocTreeNode;
  depth: number;
  selectedPath: string | null;
  expandedFolders: Set<string>;
  renamingPath: string | null;
  renameValue: string;
  dragOverPath: string | null;
}>();

const emit = defineEmits<{
  select: [node: DocTreeNode];
  renameStart: [node: DocTreeNode];
  renameCommit: [];
  renameCancel: [];
  renameUpdate: [value: string];
  deleteFolder: [node: DocTreeNode];
  deleteFile: [node: DocTreeNode];
  createSubfolder: [path: string];
  contextMenu: [event: { x: number; y: number; node: DocTreeNode }];
  pointerDragStart: [event: PointerEvent, node: DocTreeNode];
}>();

const isFolder = computed(() => props.node.nodeType === "folder");
const isExpanded = computed(() => props.expandedFolders.has(props.node.path));
const isSelected = computed(() => props.selectedPath === props.node.path);
const isRenaming = computed(() => props.renamingPath === props.node.path);
const isDragOver = computed(() => props.dragOverPath === props.node.path);

function handleClick() {
  emit("select", props.node);
}

function handleContext(e: MouseEvent) {
  e.preventDefault();
  emit("contextMenu", { x: e.clientX, y: e.clientY, node: props.node });
}

function handleMoreClick(e: MouseEvent) {
  e.stopPropagation();
  emit("contextMenu", { x: e.clientX, y: e.clientY, node: props.node });
}

function handleRenameInput(e: Event) {
  emit("renameUpdate", (e.target as HTMLInputElement).value);
}

function handleRenameKeyup(e: KeyboardEvent) {
  if (e.key === "Enter") emit("renameCommit");
  if (e.key === "Escape") emit("renameCancel");
}

function handlePointerDown(e: PointerEvent) {
  if (isFolder.value || isRenaming.value || e.button !== 0) return;
  emit("pointerDragStart", e, props.node);
}
</script>

<template>
  <div>
    <button
      class="w-full text-left flex items-center gap-1.5 transition-all text-[12px] group"
      :class="{ 'drag-over': isDragOver }"
      :data-doc-path="node.path"
      :data-doc-folder="isFolder ? node.path : undefined"
      :style="{
        paddingLeft: `${depth * 16 + 16}px`,
        paddingRight: '8px',
        paddingTop: '6px',
        paddingBottom: '6px',
        background: isDragOver ? 'var(--surface-highlight)' : (isSelected ? 'var(--sidebar-accent)' : 'transparent'),
        color: isSelected ? 'var(--foreground)' : 'var(--muted-foreground)',
        border: isDragOver ? '1px dashed var(--accent-blue)' : '1px solid transparent',
      }"
      @click="handleClick"
      @contextmenu="handleContext"
      draggable="false"
      @dragstart.prevent
      @pointerdown="handlePointerDown"
    >
      <span v-if="isFolder" class="flex-shrink-0 w-3 flex items-center justify-center">
        <ChevronDown v-if="isExpanded" class="h-3 w-3" />
        <ChevronRight v-else class="h-3 w-3" />
      </span>
      <span v-else class="w-3 flex-shrink-0" />

      <Folder v-if="isFolder" class="h-3.5 w-3.5 flex-shrink-0" />
      <FileText v-else class="h-3.5 w-3.5 flex-shrink-0" />

      <input
        v-if="isRenaming"
        :value="renameValue"
        class="flex-1 bg-transparent border-b border-[var(--border)] outline-none text-[12px] px-0.5 min-w-0"
        style="color: var(--foreground)"
        @input="handleRenameInput"
        @keyup="handleRenameKeyup"
        @click.stop
        autofocus
      />
      <span v-else class="truncate flex-1 min-w-0">{{ node.name }}</span>

      <Pencil
        v-if="!isRenaming"
        class="h-3 w-3 flex-shrink-0 cursor-pointer"
        style="color: var(--muted-foreground)"
        @click.stop="emit('renameStart', node)"
      />
      <MoreVertical
        v-if="isFolder"
        class="h-3 w-3 flex-shrink-0 opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer"
        @click="handleMoreClick"
      />
    </button>

    <!-- Children -->
    <div v-if="isFolder && isExpanded && node.children">
      <DocTreeItem
        v-for="child in node.children"
        :key="child.path"
        :node="child"
        :depth="depth + 1"
        :selected-path="selectedPath"
        :expanded-folders="expandedFolders"
        :renaming-path="renamingPath"
        :rename-value="renameValue"
        :drag-over-path="dragOverPath"
        @select="emit('select', $event)"
        @rename-start="emit('renameStart', $event)"
        @rename-commit="emit('renameCommit')"
        @rename-cancel="emit('renameCancel')"
        @rename-update="emit('renameUpdate', $event)"
        @delete-folder="emit('deleteFolder', $event)"
        @delete-file="emit('deleteFile', $event)"
        @create-subfolder="emit('createSubfolder', $event)"
        @context-menu="emit('contextMenu', $event)"
        @pointer-drag-start="(event, node) => emit('pointerDragStart', event, node)"
      />
    </div>
  </div>
</template>

<style scoped>
.drag-over {
  animation: pulse 1s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.7; }
}
</style>
