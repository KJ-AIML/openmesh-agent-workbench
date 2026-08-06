<script setup lang="ts">
// Isolated composer so keystrokes don't re-render the message thread /
// session rail. Local input state stays here; parent only hears send/stop.
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import {
  BookOpen,
  ChevronDown,
  Command,
  Diff,
  FileText,
  FolderGit2,
  FolderOpen,
  LayoutPanelTop,
  ListChecks,
  PlayCircle,
  Search,
  Send,
  Square,
  SquareTerminal,
  StickyNote,
  Wrench,
} from "lucide-vue-next";
import {
  buildMentionMenuItems,
  buildSlashMenuItems,
  filterMentionMenuItems,
  filterSlashMenuItems,
  matchMentionToken,
  matchSlashToken,
  replaceToken,
  type MentionMenuItem,
  type SlashMenuIcon,
  type SlashMenuItem,
} from "../../lib/agentChat/composerMenus";
import type { SessionRun } from "../../lib/agentChat/sessionRuns";
import type { ShellTab } from "../../lib/agentChat/shellTabs";
import { store } from "../../lib/store";
import { runAgentWorkspaceTool } from "../../lib/agentEngineClient";

export type ComposerMode = "ask" | "plan" | "act" | "delegate";

const MODES: readonly ComposerMode[] = ["ask", "plan", "act", "delegate"] as const;

const ALL_SLASH = buildSlashMenuItems();

const props = defineProps<{
  busy: boolean;
  mode: ComposerMode;
  projectPath?: string | null;
  projectName?: string | null;
  terminalRuns?: SessionRun[];
  shellTabs?: ShellTab[];
  showCanvas?: boolean;
}>();

const emit = defineEmits<{
  send: [text: string];
  stop: [];
  "update:mode": [mode: ComposerMode];
  tools: [];
  "open-canvas": [];
  "select-terminal": [run: SessionRun];
  "focus-shell": [id: string];
  "open-terminal-panel": [];
}>();

const input = ref("");
const textareaEl = ref<HTMLTextAreaElement | null>(null);
const commandsOpen = ref(false);
const mentionsOpen = ref(false);
const commandsBtnEl = ref<HTMLButtonElement | null>(null);
const commandsMenuEl = ref<HTMLElement | null>(null);
const mentionsMenuEl = ref<HTMLElement | null>(null);
const modeOpen = ref(false);
const modeBtnEl = ref<HTMLButtonElement | null>(null);
const modeMenuEl = ref<HTMLElement | null>(null);
const selectedIndex = ref(0);
const mentionFiles = ref<string[]>([]);
const mentionDocs = ref<{ name: string; path: string }[]>([]);
const mentionNotes = ref<{ name: string; path: string }[]>([]);
let mentionLoadToken = 0;

const canSend = computed(() => !!input.value.trim() && !props.busy);

const slashQuery = computed(() => {
  const m = matchSlashToken(input.value);
  return m?.query ?? "";
});

const mentionQuery = computed(() => {
  const m = matchMentionToken(input.value);
  return m?.query ?? "";
});

const slashItems = computed(() =>
  filterSlashMenuItems(ALL_SLASH, slashQuery.value),
);

const mentionItems = computed(() =>
  filterMentionMenuItems(
    buildMentionMenuItems({
      projectPath: props.projectPath,
      projectName: props.projectName,
      files: mentionFiles.value,
      docs: mentionDocs.value,
      notes: mentionNotes.value,
      terminalRuns: props.terminalRuns,
      shellTabs: props.shellTabs,
      showCanvas: props.showCanvas !== false,
    }),
    mentionQuery.value,
  ),
);

const activeMenuItems = computed(() =>
  mentionsOpen.value ? mentionItems.value : slashItems.value,
);

watch([slashItems, mentionItems, commandsOpen, mentionsOpen], () => {
  selectedIndex.value = 0;
});

function submit() {
  const text = input.value.trim();
  if (!text || props.busy) return;
  closeMenus();
  emit("send", text);
  input.value = "";
}

function closeMenus() {
  commandsOpen.value = false;
  mentionsOpen.value = false;
  modeOpen.value = false;
}

function setText(text: string) {
  input.value = text;
}

function focus() {
  textareaEl.value?.focus();
}

function insertSlash(cmd: string) {
  input.value = cmd === "/tools" || cmd === "/help" ? cmd : cmd.endsWith(" ") ? cmd : `${cmd} `;
  closeMenus();
  void nextTick(() => textareaEl.value?.focus());
}

async function loadMentionContext() {
  const path = props.projectPath?.trim();
  if (!path) {
    mentionFiles.value = [];
    mentionDocs.value = [];
    mentionNotes.value = [];
    return;
  }
  const token = ++mentionLoadToken;
  try {
    const [docs, notes, listOut] = await Promise.all([
      store.listDocs(path).catch(() => []),
      store.listNotes(path).catch(() => []),
      runAgentWorkspaceTool(path, "list_dir", { path: "." }).catch(() => ""),
    ]);
    if (token !== mentionLoadToken) return;
    mentionDocs.value = (docs ?? []).map((d) => ({
      name: d.name ?? d.path,
      path: d.path ?? d.name,
    }));
    mentionNotes.value = (notes ?? []).map((n) => ({
      name: n.name ?? n.path,
      path: n.path ?? n.name,
    }));
    mentionFiles.value = parseListDirNames(String(listOut ?? ""));
  } catch {
    if (token !== mentionLoadToken) return;
    mentionFiles.value = [];
    mentionDocs.value = [];
    mentionNotes.value = [];
  }
}

function parseListDirNames(raw: string): string[] {
  const names: string[] = [];
  try {
    const parsed = JSON.parse(raw) as unknown;
    if (Array.isArray(parsed)) {
      for (const entry of parsed) {
        if (typeof entry === "string") names.push(entry);
        else if (entry && typeof entry === "object") {
          const o = entry as { name?: string; path?: string; is_dir?: boolean };
          if (o.is_dir) continue;
          const n = o.path || o.name;
          if (n) names.push(n);
        }
      }
      return names.slice(0, 40);
    }
  } catch {
    /* plain text lines */
  }
  for (const line of raw.split("\n")) {
    const t = line.replace(/^[-*]\s*/, "").trim();
    if (!t || t.endsWith("/") || t.startsWith("dir:") || t.startsWith("…")) {
      continue;
    }
    if (t.startsWith("{") || t.startsWith("[")) continue;
    names.push(t.split(/\s+/)[0]!);
  }
  return names.slice(0, 40);
}

function onInput() {
  const slash = matchSlashToken(input.value);
  const mention = matchMentionToken(input.value);

  if (mention && !slash) {
    mentionsOpen.value = true;
    commandsOpen.value = false;
    modeOpen.value = false;
    void loadMentionContext();
  } else if (slash && input.value.trimStart().startsWith("/")) {
    // Only auto-open slash menu when the buffer is a slash command draft
    // (starts with /) — avoids fighting mid-sentence paths.
    const onlySlash = /^\s*\/[a-z-]*$/i.test(input.value);
    if (onlySlash || input.value === "/") {
      commandsOpen.value = true;
      mentionsOpen.value = false;
      modeOpen.value = false;
    } else if (commandsOpen.value && !onlySlash) {
      commandsOpen.value = false;
    }
  } else {
    if (commandsOpen.value && !slash) commandsOpen.value = false;
    if (mentionsOpen.value && !mention) mentionsOpen.value = false;
  }
}

function toggleCommands() {
  commandsOpen.value = !commandsOpen.value;
  if (commandsOpen.value) {
    mentionsOpen.value = false;
    modeOpen.value = false;
    if (!input.value.startsWith("/")) {
      // Keep input as-is; show full inventory
    }
    void nextTick(() => commandsMenuEl.value?.focus());
  }
}

function closeCommands() {
  commandsOpen.value = false;
  commandsBtnEl.value?.focus();
}

function toggleModeMenu() {
  modeOpen.value = !modeOpen.value;
  if (modeOpen.value) {
    commandsOpen.value = false;
    mentionsOpen.value = false;
    void nextTick(() => modeMenuEl.value?.focus());
  }
}

function selectMode(m: ComposerMode) {
  emit("update:mode", m);
  modeOpen.value = false;
  void nextTick(() => textareaEl.value?.focus());
}

function pickSlash(item: SlashMenuItem) {
  if (item.slash === "/tools" || item.slash === "/help") {
    commandsOpen.value = false;
    if (matchSlashToken(input.value) && /^\s*\/[a-z-]*$/i.test(input.value)) {
      input.value = "";
    }
    emit("tools");
    return;
  }
  const token = matchSlashToken(input.value);
  if (token && /^\s*\/[a-z-]*$/i.test(input.value)) {
    input.value = item.insert;
  } else if (token) {
    input.value = replaceToken(
      input.value,
      token.start,
      token.start + token.query.length,
      item.insert.trimEnd(),
    );
  } else {
    input.value = item.insert;
  }
  commandsOpen.value = false;
  void nextTick(() => textareaEl.value?.focus());
}

function pickMention(item: MentionMenuItem) {
  const token = matchMentionToken(input.value);
  if (item.insert && token) {
    const needsSpace = !item.insert.endsWith(" ");
    input.value = replaceToken(
      input.value,
      token.start,
      token.start + token.query.length,
      needsSpace ? `${item.insert} ` : item.insert,
    );
  } else if (item.insert && !token) {
    input.value = item.insert.endsWith(" ") ? item.insert : `${item.insert} `;
  } else if (token && !item.insert) {
    // Action-only: remove the @token
    input.value = replaceToken(
      input.value,
      token.start,
      token.start + token.query.length,
      "",
    ).replace(/\s{2,}/g, " ");
  }

  mentionsOpen.value = false;

  if (item.action === "open-canvas") emit("open-canvas");
  if (item.action === "open-terminal-panel") emit("open-terminal-panel");
  if (item.action === "focus-shell" && item.actionId) {
    emit("focus-shell", item.actionId);
  }
  if (item.action === "select-terminal" && item.actionId) {
    const run = (props.terminalRuns ?? []).find((r) => r.id === item.actionId);
    if (run) emit("select-terminal", run);
  }

  void nextTick(() => textareaEl.value?.focus());
}

function listAllTools() {
  commandsOpen.value = false;
  if (/^\s*\/[a-z-]*$/i.test(input.value)) input.value = "";
  emit("tools");
}

function moveSelection(delta: number) {
  const items = activeMenuItems.value;
  if (!items.length) return;
  const next = selectedIndex.value + delta;
  selectedIndex.value = ((next % items.length) + items.length) % items.length;
  void nextTick(() => {
    const root = mentionsOpen.value ? mentionsMenuEl.value : commandsMenuEl.value;
    const el = root?.querySelector<HTMLElement>(
      `[data-menu-index="${selectedIndex.value}"]`,
    );
    el?.scrollIntoView({ block: "nearest" });
  });
}

function confirmSelection() {
  if (mentionsOpen.value) {
    const item = mentionItems.value[selectedIndex.value];
    if (item) pickMention(item);
    return;
  }
  if (commandsOpen.value) {
    const item = slashItems.value[selectedIndex.value];
    if (item) pickSlash(item);
  }
}

function onTextareaKeydown(e: KeyboardEvent) {
  const menuOpen = commandsOpen.value || mentionsOpen.value;
  if (!menuOpen) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
    return;
  }
  if (e.key === "ArrowDown") {
    e.preventDefault();
    moveSelection(1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    moveSelection(-1);
  } else if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    confirmSelection();
  } else if (e.key === "Escape") {
    e.preventDefault();
    closeMenus();
  } else if (e.key === "Tab") {
    e.preventDefault();
    confirmSelection();
  }
}

function onGlobalKeydown(e: KeyboardEvent) {
  if (e.key !== "Escape") return;
  if (commandsOpen.value || mentionsOpen.value || modeOpen.value) {
    e.preventDefault();
    closeMenus();
    if (!commandsOpen.value && !mentionsOpen.value) {
      // mode was open
    }
    modeOpen.value = false;
  }
}

watch([commandsOpen, mentionsOpen, modeOpen], ([cmd, mention, mode]) => {
  if (cmd || mention || mode) window.addEventListener("keydown", onGlobalKeydown);
  else window.removeEventListener("keydown", onGlobalKeydown);
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
});

function slashIcon(icon: SlashMenuIcon) {
  switch (icon) {
    case "pilot":
      return PlayCircle;
    case "read":
      return FileText;
    case "diff":
      return Diff;
    case "verify":
      return ListChecks;
    case "continue":
      return PlayCircle;
    case "search":
      return Search;
    case "git":
      return FolderGit2;
    case "tools":
      return Wrench;
    default:
      return Command;
  }
}

function mentionIcon(kind: MentionMenuItem["kind"]) {
  switch (kind) {
    case "project":
      return FolderOpen;
    case "file":
      return FileText;
    case "doc":
      return BookOpen;
    case "note":
      return StickyNote;
    case "terminal":
    case "shell":
      return SquareTerminal;
    case "canvas":
      return LayoutPanelTop;
    default:
      return Command;
  }
}

defineExpose({ insertSlash, setText, focus });
</script>

<template>
  <footer class="chat-composer" data-testid="chat-composer">
    <div class="chat-composer__shell">
      <textarea
        ref="textareaEl"
        v-model="input"
        class="chat-composer__input"
        rows="2"
        placeholder="Ask the workspace…  (/ commands · @ context)"
        aria-label="Message"
        @input="onInput"
        @keydown="onTextareaKeydown"
      />

      <div class="chat-composer__toolbar" role="toolbar" aria-label="Composer">
        <div class="chat-composer__mode-wrap">
          <button
            ref="modeBtnEl"
            type="button"
            class="chat-composer__mode"
            data-testid="composer-mode"
            :disabled="busy"
            :aria-expanded="modeOpen"
            aria-haspopup="listbox"
            :title="`Mode: ${mode}`"
            @click="toggleModeMenu"
          >
            <span class="chat-composer__mode-label">{{ mode }}</span>
            <ChevronDown :size="12" aria-hidden="true" />
          </button>
          <ul
            v-if="modeOpen"
            ref="modeMenuEl"
            class="chat-composer__menu"
            role="listbox"
            aria-label="Agent mode"
            tabindex="-1"
          >
            <li v-for="m in MODES" :key="m" role="option" :aria-selected="mode === m">
              <button
                type="button"
                class="chat-composer__menu-item"
                :class="{ 'is-active': mode === m }"
                :disabled="busy"
                @click="selectMode(m)"
              >
                {{ m }}
              </button>
            </li>
          </ul>
        </div>

        <div class="chat-composer__cmd-wrap">
          <button
            ref="commandsBtnEl"
            type="button"
            class="chat-composer__icon-btn"
            data-testid="composer-commands"
            :disabled="busy"
            :aria-expanded="commandsOpen"
            aria-haspopup="menu"
            title="Commands"
            aria-label="Commands"
            @click="toggleCommands"
          >
            <Command :size="14" aria-hidden="true" />
            <span class="chat-composer__cmd-text">/</span>
          </button>
          <div
            v-if="commandsOpen"
            ref="commandsMenuEl"
            class="chat-composer__menu chat-composer__menu--rich"
            role="menu"
            aria-label="Slash commands"
            data-testid="composer-slash-menu"
            tabindex="-1"
          >
            <button
              v-for="(item, idx) in slashItems"
              :key="item.id"
              type="button"
              class="chat-composer__rich-item"
              :class="{ 'is-active': idx === selectedIndex }"
              role="menuitem"
              :data-menu-index="idx"
              @click="pickSlash(item)"
              @mouseenter="selectedIndex = idx"
            >
              <component
                :is="slashIcon(item.icon)"
                :size="14"
                class="chat-composer__rich-icon"
                aria-hidden="true"
              />
              <span class="chat-composer__rich-text">
                <span class="chat-composer__rich-label">
                  <span class="chat-composer__rich-slash">{{ item.slash }}</span>
                  <span class="chat-composer__rich-name">{{ item.label }}</span>
                </span>
                <span class="chat-composer__rich-desc">{{ item.description }}</span>
              </span>
            </button>
            <button
              type="button"
              class="chat-composer__menu-item chat-composer__menu-item--more"
              role="menuitem"
              @click="listAllTools"
            >
              All tools…
            </button>
          </div>
        </div>

        <div class="chat-composer__status">
          <slot name="status" />
        </div>

        <div class="chat-composer__spacer" aria-hidden="true" />

        <button
          v-if="busy"
          type="button"
          class="btn-secondary chat-composer__send"
          data-testid="composer-stop"
          @click="emit('stop')"
        >
          <Square :size="14" />
          Stop
        </button>
        <button
          v-else
          type="button"
          class="btn-primary chat-composer__send"
          data-testid="composer-send"
          :disabled="!canSend"
          @click="submit"
        >
          <Send :size="15" />
          Send
        </button>
      </div>
    </div>

    <div
      v-if="mentionsOpen"
      ref="mentionsMenuEl"
      class="chat-composer__menu chat-composer__menu--rich chat-composer__menu--mentions"
      role="menu"
      aria-label="Context mentions"
      data-testid="composer-mention-menu"
      tabindex="-1"
    >
      <button
        v-for="(item, idx) in mentionItems"
        :key="item.id"
        type="button"
        class="chat-composer__rich-item"
        :class="{ 'is-active': idx === selectedIndex }"
        role="menuitem"
        :data-menu-index="idx"
        @click="pickMention(item)"
        @mouseenter="selectedIndex = idx"
      >
        <component
          :is="mentionIcon(item.kind)"
          :size="14"
          class="chat-composer__rich-icon"
          aria-hidden="true"
        />
        <span class="chat-composer__rich-text">
          <span class="chat-composer__rich-label">
            <span class="chat-composer__rich-name">{{ item.label }}</span>
            <span class="chat-composer__rich-kind">{{ item.kind }}</span>
          </span>
          <span class="chat-composer__rich-desc">{{ item.description }}</span>
        </span>
      </button>
      <p v-if="mentionItems.length === 0" class="chat-composer__menu-empty">
        No matching context
      </p>
    </div>
  </footer>
</template>

<style scoped>
.chat-composer {
  flex-shrink: 0;
  position: relative;
}

.chat-composer__shell {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--surface-2);
  transition:
    border-color 0.15s ease,
    box-shadow 0.15s ease,
    background 0.15s ease;
}

.chat-composer__shell:focus-within {
  border-color: var(--border-strong);
  background: var(--surface-3);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-blue) 16%, transparent);
}

.chat-composer__input {
  display: block;
  width: 100%;
  resize: vertical;
  min-height: 52px;
  max-height: 160px;
  border: none;
  border-radius: 12px 12px 0 0;
  background: transparent;
  color: var(--foreground);
  padding: 0.75rem 0.85rem 0.35rem;
  font: inherit;
  font-size: 0.875rem;
  line-height: 1.45;
}

.chat-composer__input:focus {
  outline: none;
}

.chat-composer__input::placeholder {
  color: var(--muted-foreground);
  opacity: 0.85;
}

.chat-composer__toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.3rem;
  padding: 0.3rem 0.4rem 0.4rem;
  border-top: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
}

.chat-composer__mode-wrap,
.chat-composer__cmd-wrap {
  position: relative;
}

.chat-composer__mode {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  min-height: 28px;
  padding: 0.15rem 0.45rem 0.15rem 0.55rem;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--muted-foreground);
  font: inherit;
  font-size: 0.72rem;
  font-weight: 550;
  text-transform: capitalize;
  letter-spacing: -0.01em;
  cursor: pointer;
  transition:
    background 0.12s ease,
    border-color 0.12s ease,
    color 0.12s ease;
}

.chat-composer__mode:hover:not(:disabled),
.chat-composer__mode[aria-expanded="true"] {
  background: var(--surface-hover);
  border-color: var(--border);
  color: var(--foreground);
}

.chat-composer__mode:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.chat-composer__mode-label {
  max-width: 5.5rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chat-composer__icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.1rem;
  min-width: 28px;
  min-height: 28px;
  padding: 0 0.4rem;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--muted-foreground);
  cursor: pointer;
  transition:
    background 0.12s ease,
    border-color 0.12s ease,
    color 0.12s ease;
}

.chat-composer__icon-btn:hover:not(:disabled),
.chat-composer__icon-btn[aria-expanded="true"] {
  background: var(--surface-hover);
  border-color: var(--border);
  color: var(--foreground);
}

.chat-composer__icon-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.chat-composer__cmd-text {
  font-size: 0.72rem;
  font-weight: 600;
  font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace);
}

.chat-composer__status {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  min-width: 0;
}

.chat-composer__spacer {
  flex: 1 1 0.5rem;
  min-width: 0.25rem;
}

.chat-composer__send {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  min-height: 30px;
  padding-inline: 0.7rem;
  font-size: 0.78rem;
}

.chat-composer__menu {
  position: absolute;
  left: 0;
  bottom: calc(100% + 0.35rem);
  z-index: 45;
  margin: 0;
  padding: 0.3rem;
  list-style: none;
  min-width: 8.5rem;
  border-radius: 10px;
  border: 1px solid var(--border-strong);
  background: var(--surface-2);
  box-shadow:
    0 10px 28px color-mix(in srgb, #000 40%, transparent),
    0 0 0 1px color-mix(in srgb, var(--border) 50%, transparent);
  outline: none;
}

.chat-composer__menu--rich {
  min-width: min(22rem, calc(100vw - 2rem));
  max-width: min(26rem, calc(100vw - 2rem));
  max-height: min(320px, 45vh);
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 0.08rem;
  padding: 0.35rem;
}

.chat-composer__menu--mentions {
  left: 0.4rem;
  right: 0.4rem;
  bottom: calc(100% + 0.35rem);
  min-width: 0;
  max-width: none;
  width: auto;
}

.chat-composer__rich-item {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.55rem;
  align-items: start;
  width: 100%;
  text-align: left;
  border: 1px solid transparent;
  border-radius: 8px;
  background: transparent;
  color: var(--foreground);
  padding: 0.42rem 0.5rem;
  font: inherit;
  cursor: pointer;
}

.chat-composer__rich-item:hover,
.chat-composer__rich-item.is-active {
  background: color-mix(in srgb, var(--accent-blue) 14%, var(--surface-3));
  border-color: color-mix(in srgb, var(--accent-blue) 30%, var(--border));
}

.chat-composer__rich-icon {
  margin-top: 0.12rem;
  color: var(--muted-foreground);
  flex-shrink: 0;
}

.chat-composer__rich-item.is-active .chat-composer__rich-icon {
  color: var(--foreground);
}

.chat-composer__rich-text {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.12rem;
}

.chat-composer__rich-label {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.35rem;
}

.chat-composer__rich-slash {
  font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace);
  font-size: 0.74rem;
  font-weight: 600;
}

.chat-composer__rich-name {
  font-size: 0.78rem;
  font-weight: 600;
}

.chat-composer__rich-kind {
  font-size: 0.62rem;
  font-weight: 550;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--muted-foreground);
}

.chat-composer__rich-desc {
  font-size: 0.68rem;
  color: var(--muted-foreground);
  line-height: 1.35;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.chat-composer__menu-item {
  display: block;
  width: 100%;
  text-align: left;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  color: var(--foreground);
  padding: 0.4rem 0.55rem;
  font: inherit;
  font-size: 0.78rem;
  text-transform: capitalize;
  cursor: pointer;
}

.chat-composer__menu-item:hover,
.chat-composer__menu-item.is-active {
  background: var(--surface-3);
  border-color: var(--border);
}

.chat-composer__menu-item--more {
  margin-top: 0.15rem;
  border-top: 1px solid var(--border);
  border-radius: 0 0 7px 7px;
  font-family: inherit !important;
  color: var(--muted-foreground);
  text-transform: none;
}

.chat-composer__menu-empty {
  margin: 0;
  padding: 0.55rem 0.5rem;
  font-size: 0.72rem;
  color: var(--muted-foreground);
}

.chat-composer__menu-item:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

@media (max-width: 520px) {
  .chat-composer__cmd-text {
    display: none;
  }

  .chat-composer__send {
    padding-inline: 0.55rem;
  }
}

html[data-density="compact"] .chat-composer__input {
  min-height: 44px;
  padding: 0.55rem 0.7rem 0.25rem;
  font-size: 0.82rem;
}

html[data-density="compact"] .chat-composer__toolbar {
  padding: 0.22rem 0.3rem 0.3rem;
}
</style>
