<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  shallowRef,
  triggerRef,
  watch,
} from "vue";
import { useRouter } from "vue-router";
import {
  AlertCircle,
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Copy,
  Eraser,
  GitFork,
  Pencil,
  Plus,
  Settings,
  Sparkles,
  Trash2,
  Wrench,
} from "lucide-vue-next";
import { useStore } from "../lib/useStore";
import {
  runAgentChatTurn,
  type ChatTurnProgress,
} from "../lib/agentChat/runner";
import {
  isToolsHelpText,
  resolveToolsForMessage,
  summarizeToolsHelp,
} from "../lib/agentChat/tools";
import {
  chatModelId,
  getChatSetupChecks,
  isChatProviderReady,
} from "../lib/agentChat/ready";
import {
  cancelAgentEngineTurn,
  extractPatchIds,
  getAgentSecretStatus,
  type AgentToolStep,
} from "../lib/agentEngineClient";
import {
  applyVoiceUiActions,
  parseUiActionsFromToolSteps,
} from "../lib/voice/uiActions";
import {
  clearAppActionHandlers,
  registerAppActionHandlers,
} from "../lib/appActions/dispatcher";
import { setAppContext } from "../lib/appActions/context";
import { registerVoiceChatLink } from "../lib/agentChat/voiceBridge";
import {
  type ChatMessage,
  type ChatSession,
  createChatMessage,
  createChatSession,
  deriveTitleFromMessage,
  forkSessionAt,
  isAutoTitleWorthy,
  loadSessionsAsync,
  persistSessions,
  persistSessionsAsync,
  touchSession,
} from "../lib/agentChat/chatSessions";
import { createPersistQueue } from "../lib/agentChat/persistQueue";
import ChatMessageContent from "../components/chat/ChatMessageContent.vue";
import ChatComposer from "../components/chat/ChatComposer.vue";
import ChatThinkingBubble from "../components/chat/ChatThinkingBubble.vue";
import PatchApprovalCard from "../components/chat/PatchApprovalCard.vue";
import VerifyLogPanel from "../components/chat/VerifyLogPanel.vue";

type ChatMode = "ask" | "plan" | "act" | "delegate";

const router = useRouter();
const { currentProjectPath, currentProject, settings, saveSettings } = useStore();

/** Real secret-store presence; null until probed (or when IPC unavailable). */
const secretConfigured = ref<boolean | null>(null);

/** Shallow — avoid deep-watching the whole session tree on every keystroke/send. */
const sessions = shallowRef<ChatSession[]>([]);
/** Live message array for the active chat; bumped via triggerRef on mutate. */
const activeMessages = shallowRef<ChatMessage[]>([]);
const activeSessionId = ref<string | null>(null);
const renamingId = ref<string | null>(null);
const renameValue = ref("");

const busy = ref(false);
/** Primary status line for the in-thread thinking bubble. */
const busyLabel = ref("Thinking…");
/** Compact mid-turn tool line (e.g. "Pilot status"). */
const busyDetail = ref<string | null>(null);
const error = ref<string | null>(null);
const chatMode = ref<ChatMode>("ask");
const activeTurnId = ref<string | null>(null);
const verifyRunKey = ref<string | null>(null);
const scroller = ref<HTMLElement | null>(null);
const composer = ref<InstanceType<typeof ChatComposer> | null>(null);
/** Expanded /tools dump message ids (collapsed by default). */
const expandedToolsIds = ref<Set<string>>(new Set());
/** Brief “Copied” feedback on the message action that succeeded. */
const copiedMessageId = ref<string | null>(null);
let copiedClearTimer: ReturnType<typeof setTimeout> | null = null;
/** Lightweight toast for copy / fork feedback. */
const actionToast = ref("");
let actionToastTimer: ReturnType<typeof setTimeout> | null = null;

/** Rotating CLI-style status while waiting on the LLM (no streaming events). */
const LLM_STATUS_LINES = [
  "Thinking…",
  "Reasoning…",
  "Working with tools…",
  "Composing reply…",
] as const;
let statusRotateTimer: ReturnType<typeof setInterval> | null = null;
let statusRotateIdx = 0;

const persistQueue = createPersistQueue<ChatSession>((path, list) => {
  persistSessions(path, list);
  void persistSessionsAsync(path, list);
}, 400);

const hasProject = computed(() => !!currentProjectPath.value);
const projectLabel = computed(
  () => currentProject.value?.name || currentProjectPath.value || "No project",
);
const chatReadyOpts = computed(() =>
  typeof secretConfigured.value === "boolean"
    ? { secretConfigured: secretConfigured.value }
    : undefined,
);
const chatReady = computed(() =>
  isChatProviderReady(settings.value, chatReadyOpts.value),
);
const setupChecks = computed(() =>
  getChatSetupChecks(settings.value, chatReadyOpts.value),
);
const activeModel = computed(() => chatModelId(settings.value));

async function syncSecretConfigured() {
  try {
    const status = await getAgentSecretStatus();
    secretConfigured.value = status.configured;
    const flagged = !!settings.value.provider?.apiKeyConfigured;
    if (flagged !== status.configured && settings.value.provider) {
      await saveSettings({
        provider: {
          ...settings.value.provider,
          apiKeyConfigured: status.configured,
        },
      });
    }
  } catch {
    // Browser / mock — keep using settings JSON flag.
    secretConfigured.value = null;
  }
}

const activeSession = computed(
  () => sessions.value.find((s) => s.id === activeSessionId.value) ?? null,
);

function welcomeText(): string {
  const provider = settings.value.provider?.name?.trim() || "provider";
  return (
    `OpenMesh Agent Engine · ${projectLabel.value}\n` +
    `Provider: ${provider} · Model: ${activeModel.value}`
  );
}

function seedWelcome(session: ChatSession) {
  session.messages.push(createChatMessage("system", welcomeText()));
}

/** Bind the thread view to a session's messages array (same ref; trigger to refresh). */
function bindActiveMessages(session: ChatSession | null) {
  activeMessages.value = session?.messages ?? [];
}

/** After mutating session/message data — refresh shallow refs + schedule idle persist. */
function afterSessionMutation(session?: ChatSession | null) {
  triggerRef(sessions);
  const active =
    session && session.id === activeSessionId.value
      ? session
      : (sessions.value.find((s) => s.id === activeSessionId.value) ?? null);
  if (active) {
    if (activeMessages.value !== active.messages) {
      activeMessages.value = active.messages;
    } else {
      triggerRef(activeMessages);
    }
  } else {
    activeMessages.value = [];
  }
  if (currentProjectPath.value) {
    persistQueue.schedule(currentProjectPath.value, sessions.value);
  }
}

async function loadForProject(path: string) {
  const loaded = await loadSessionsAsync(path);
  if (loaded.length === 0) {
    const fresh = createChatSession();
    seedWelcome(fresh);
    sessions.value = [fresh];
    activeSessionId.value = fresh.id;
    bindActiveMessages(fresh);
    // Persist the seeded welcome once (idle) so reload keeps it.
    persistQueue.schedule(path, sessions.value);
  } else {
    sessions.value = loaded;
    activeSessionId.value = loaded[0].id;
    bindActiveMessages(loaded[0]);
  }
}

watch(
  [currentProjectPath, chatReady],
  ([path, ready]) => {
    // Flush any pending write for the previous project before swapping state.
    persistQueue.flush();
    error.value = null;
    renamingId.value = null;
    if (path && ready) void loadForProject(path);
    else {
      sessions.value = [];
      activeSessionId.value = null;
      activeMessages.value = [];
    }
  },
  { immediate: true },
);

onMounted(() => {
  void syncSecretConfigured();
  registerAppActionHandlers({
    setComposer: (text) => composer.value?.setText(text),
    focusComposer: () => composer.value?.focus(),
    setMode: (mode) => {
      if (mode === "ask" || mode === "plan" || mode === "act" || mode === "delegate") {
        chatMode.value = mode;
      }
    },
    selectSession: (sessionId) => switchToSession(sessionId),
  });
  registerVoiceChatLink({
    getHistory: () => {
      const session = activeSession.value;
      if (!session) return [];
      return session.messages
        .filter((m) => m.role === "user" || m.role === "assistant")
        .slice(-12)
        .map((m) => ({ role: m.role, content: m.text }));
    },
    getMode: () => chatMode.value,
    getSettings: () => settings.value,
    appendExchange: (userText, result) => {
      const session = activeSession.value;
      if (!session) return;
      session.messages.push(createChatMessage("user", userText));
      if (session.titleIsDefault && isAutoTitleWorthy(userText)) {
        session.title = deriveTitleFromMessage(userText);
        session.titleIsDefault = false;
      }
      session.messages.push(
        createChatMessage("assistant", result.assistantText, result.toolCalls),
      );
      touchSession(session);
      afterSessionMutation(session);
      void scrollBottom();
    },
  });
  setAppContext({
    route: router.currentRoute.value.path,
    chatMode: chatMode.value,
    activeSessionId: activeSessionId.value ?? undefined,
    projectPath: currentProjectPath.value ?? undefined,
  });
});

watch(
  () => settings.value.provider?.apiKeyConfigured,
  () => {
    void syncSecretConfigured();
  },
);

watch(
  [chatMode, activeSessionId, currentProjectPath, () => router.currentRoute.value.path],
  () => {
    setAppContext({
      route: router.currentRoute.value.path,
      chatMode: chatMode.value,
      activeSessionId: activeSessionId.value ?? undefined,
      projectPath: currentProjectPath.value ?? undefined,
    });
  },
);

onBeforeUnmount(() => {
  stopStatusRotate();
  persistQueue.flush();
  if (copiedClearTimer !== null) clearTimeout(copiedClearTimer);
  if (actionToastTimer !== null) clearTimeout(actionToastTimer);
  registerVoiceChatLink(null);
  clearAppActionHandlers();
});

function showActionToast(msg: string) {
  actionToast.value = msg;
  if (actionToastTimer !== null) clearTimeout(actionToastTimer);
  actionToastTimer = setTimeout(() => {
    actionToast.value = "";
    actionToastTimer = null;
  }, 1600);
}

function showMessageActions(m: ChatMessage): boolean {
  return m.role === "user" || m.role === "assistant";
}

async function copyMessage(m: ChatMessage) {
  const text = m.text;
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
    copiedMessageId.value = m.id;
    if (copiedClearTimer !== null) clearTimeout(copiedClearTimer);
    copiedClearTimer = setTimeout(() => {
      copiedMessageId.value = null;
      copiedClearTimer = null;
    }, 1400);
    showActionToast("Copied");
  } catch {
    showActionToast("Copy failed");
  }
}

function forkFromMessage(messageIndex: number) {
  const session = activeSession.value;
  if (!session) return;
  const forked = forkSessionAt(session, messageIndex);
  if (!forked) return;
  sessions.value = [forked, ...sessions.value];
  activeSessionId.value = forked.id;
  bindActiveMessages(forked);
  renamingId.value = null;
  error.value = null;
  afterSessionMutation(forked);
  showActionToast("Forked chat");
  scrollBottom();
}

async function scrollBottom() {
  await nextTick();
  const el = scroller.value;
  if (el) el.scrollTop = el.scrollHeight;
}

/** Yield until the next paint so optimistic UI lands before agent work. */
function afterPaint(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve());
    });
  });
}

/** One more macrotask so the event loop can service window chrome before IPC. */
function afterEventLoop(): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, 0);
  });
}

function stopStatusRotate() {
  if (statusRotateTimer !== null) {
    clearInterval(statusRotateTimer);
    statusRotateTimer = null;
  }
  statusRotateIdx = 0;
}

function startLlmStatusRotate() {
  stopStatusRotate();
  statusRotateIdx = 0;
  busyLabel.value = LLM_STATUS_LINES[0];
  busyDetail.value = null;
  statusRotateTimer = setInterval(() => {
    statusRotateIdx = (statusRotateIdx + 1) % LLM_STATUS_LINES.length;
    busyLabel.value = LLM_STATUS_LINES[statusRotateIdx];
  }, 2400);
}

function applyTurnProgress(event: ChatTurnProgress) {
  if (event.kind === "phase") {
    busyLabel.value = event.label;
    if (event.label === "Thinking…") {
      // Keep rotating ambient lines for long LLM waits.
      if (!statusRotateTimer) startLlmStatusRotate();
    } else {
      stopStatusRotate();
      busyDetail.value = null;
    }
    return;
  }
  if (event.kind === "tool_start") {
    stopStatusRotate();
    busyLabel.value = "Working with tools…";
    busyDetail.value = event.title;
    return;
  }
  // tool_done — keep last tool visible until next event / reply lands
  busyLabel.value = event.ok ? "Finishing…" : "Tool failed…";
  busyDetail.value = event.title;
}

function startNewChat() {
  const fresh = createChatSession();
  seedWelcome(fresh);
  sessions.value = [fresh, ...sessions.value];
  activeSessionId.value = fresh.id;
  bindActiveMessages(fresh);
  renamingId.value = null;
  error.value = null;
  afterSessionMutation(fresh);
  scrollBottom();
}

function switchToSession(id: string) {
  if (id === activeSessionId.value) return;
  activeSessionId.value = id;
  bindActiveMessages(sessions.value.find((s) => s.id === id) ?? null);
  error.value = null;
  scrollBottom();
}

function beginRename(session: ChatSession) {
  renamingId.value = session.id;
  renameValue.value = session.title;
}

function commitRename() {
  const id = renamingId.value;
  renamingId.value = null;
  if (!id) return;
  const session = sessions.value.find((s) => s.id === id);
  if (!session) return;
  const trimmed = renameValue.value.trim();
  if (trimmed && trimmed !== session.title) {
    session.title = trimmed;
    session.titleIsDefault = false;
    touchSession(session);
    afterSessionMutation(session);
  }
}

function cancelRename() {
  renamingId.value = null;
}

function removeSession(id: string) {
  sessions.value = sessions.value.filter((s) => s.id !== id);
  if (activeSessionId.value === id) {
    if (sessions.value.length > 0) {
      activeSessionId.value = sessions.value[0].id;
      bindActiveMessages(sessions.value[0]);
      afterSessionMutation(sessions.value[0]);
    } else {
      startNewChat();
    }
  } else {
    afterSessionMutation();
  }
}

function clearActiveChat() {
  const session = activeSession.value;
  if (!session) return;
  session.messages = [];
  touchSession(session);
  afterSessionMutation(session);
}

function relativeTime(ts: number): string {
  const diffMs = Date.now() - ts;
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffHours / 24);
  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  return new Date(ts).toLocaleDateString();
}

async function send(text: string) {
  if (!currentProjectPath.value || busy.value || !chatReady.value) return;
  const session = activeSession.value;
  if (!session) return;
  const trimmed = text.trim();
  if (!trimmed) return;
  const projectPath = currentProjectPath.value;

  // 1) Optimistic UI only — no stringify / IPC / markdown of history here.
  // Persist is explicit + idle; composer textarea stays enabled while busy.
  session.messages.push(createChatMessage("user", trimmed));
  if (session.titleIsDefault && isAutoTitleWorthy(trimmed)) {
    session.title = deriveTitleFromMessage(trimmed);
    session.titleIsDefault = false;
  }
  touchSession(session);
  afterSessionMutation(session);

  const usesLocalTools =
    trimmed.startsWith("/") || resolveToolsForMessage(trimmed).length > 0;
  busyLabel.value = usesLocalTools ? "Working…" : "Thinking…";
  busyDetail.value = null;
  busy.value = true;
  error.value = null;
  const turnId = `turn-${Date.now().toString(16)}`;
  activeTurnId.value = turnId;
  const verifyMatch = trimmed.match(/^\/verify\s+(\S+)/i);
  verifyRunKey.value =
    verifyMatch && verifyMatch[1] && verifyMatch[1].toLowerCase() !== "list"
      ? `${projectPath}:${verifyMatch[1]}`
      : null;

  // Paint user bubble + thinking indicator, then yield the event loop before IPC.
  await scrollBottom();
  await afterPaint();
  await afterEventLoop();

  if (!usesLocalTools) startLlmStatusRotate();

  try {
    // Cheap history slice — maps a few strings only; old bubbles stay v-memo'd.
    const history = session.messages
      .filter((m) => m.role === "user" || m.role === "assistant")
      .slice(0, -1)
      .slice(-12)
      .map((m) => ({ role: m.role, content: m.text }));

    const result = await runAgentChatTurn(projectPath, trimmed, {
      settings: settings.value,
      history,
      onProgress: applyTurnProgress,
      mode: chatMode.value,
      turnId,
    });
    // Apply allowlisted in-app navigation from ui_navigate tool steps.
    const navSteps: AgentToolStep[] = (result.toolCalls ?? []).map((t) => ({
      toolName: t.toolId,
      toolCallId: t.toolId,
      ok: t.ok,
      summary: t.summary,
    }));
    await applyVoiceUiActions(router, parseUiActionsFromToolSteps(navSteps));
    session.messages.push(
      createChatMessage("assistant", result.assistantText, result.toolCalls),
    );
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
    session.messages.push(createChatMessage("assistant", `Error: ${error.value}`));
  } finally {
    stopStatusRotate();
    busy.value = false;
    busyDetail.value = null;
    activeTurnId.value = null;
    // Keep verify panel briefly after completion so logs remain visible.
    if (!verifyRunKey.value) verifyRunKey.value = null;
    touchSession(session);
    afterSessionMutation(session);
    await scrollBottom();
  }
}

/** Post-apply CTA from PatchApprovalCard — run linked verify recipe. */
async function verifyFromPatch(payload: { recipeId: string; patchId: string }) {
  const projectPath = currentProjectPath.value;
  if (!projectPath || busy.value) return;
  const runKey = `${projectPath}:${payload.recipeId}`;
  verifyRunKey.value = runKey;
  busyLabel.value = `Verifying ${payload.recipeId}…`;
  busyDetail.value = `patch ${payload.patchId}`;
  busy.value = true;
  error.value = null;
  try {
    const { runAgentRecipe } = await import("../lib/agentEngineClient");
    await runAgentRecipe(
      projectPath,
      payload.recipeId,
      runKey,
      payload.patchId,
    );
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = false;
    busyDetail.value = null;
    await scrollBottom();
  }
}

async function stopTurn() {
  const id = activeTurnId.value;
  if (id) {
    try {
      await cancelAgentEngineTurn(id);
    } catch {
      /* ignore */
    }
  }
  if (verifyRunKey.value) {
    try {
      const { cancelAgentRecipe } = await import("../lib/agentEngineClient");
      await cancelAgentRecipe(verifyRunKey.value);
    } catch {
      /* ignore */
    }
  }
  busyLabel.value = "Stopping…";
}

function insertSlash(cmd: string) {
  composer.value?.insertSlash(cmd);
}

/** Starter chips only — full catalog via “More…” → /tools (not stacked). */
const starterChips = ["/pilot", "/read", "/diff", "/verify", "/continue"] as const;

function runToolsDump() {
  void send("/tools");
}

function patchIdsForMessage(m: ChatMessage): string[] {
  const fromText = extractPatchIds(m.text);
  const fromTools = (m.toolCalls ?? [])
    .filter((t) => t.toolId === "propose_patch" || /patch-/i.test(t.summary))
    .flatMap((t) => extractPatchIds(t.summary));
  return [...new Set([...fromText, ...fromTools])];
}

function isToolsHelpMessage(m: ChatMessage): boolean {
  return m.role === "assistant" && isToolsHelpText(m.text);
}

function toolsHelpSummary(text: string): string {
  return summarizeToolsHelp(text);
}

function isToolsExpanded(id: string): boolean {
  return expandedToolsIds.value.has(id);
}

function toggleToolsExpanded(id: string) {
  const next = new Set(expandedToolsIds.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  expandedToolsIds.value = next;
}
</script>

<template>
  <div class="chat">
    <h1 class="chat__sr-only">Chat</h1>

    <div v-if="!hasProject" class="chat__viewport">
      <div class="chat__empty">
        <Sparkles :size="22" />
        <p class="chat__empty-title">Select a project first</p>
        <p class="chat__empty-body">
          Chat runs against the active workspace path.
        </p>
      </div>
    </div>

    <div v-else-if="!chatReady" class="chat__viewport">
      <div class="chat__gate workbench-card">
        <div class="chat__gate-head">
          <Settings :size="18" />
          <div>
            <h2 class="chat__gate-title">Set up provider before chat</h2>
            <p class="chat__gate-body">
              Add a provider, mark an API key as configured, and choose a default
              model. Chat stays locked until these are saved in Settings.
              Use a normal OpenAI-compatible endpoint — not DashScope Coding Plan.
            </p>
          </div>
        </div>
        <ul class="chat__gate-list">
          <li v-for="c in setupChecks" :key="c.id" class="chat__gate-item">
            <CheckCircle2
              v-if="c.done"
              class="chat__gate-icon chat__gate-icon--ok"
              :size="16"
            />
            <AlertCircle
              v-else
              class="chat__gate-icon chat__gate-icon--warn"
              :size="16"
            />
            <div>
              <div class="chat__gate-label">{{ c.label }}</div>
              <div class="chat__gate-hint">{{ c.hint }}</div>
            </div>
            <span class="chat__gate-badge" :class="c.done ? 'is-ok' : 'is-warn'">
              {{ c.done ? "Ready" : "Required" }}
            </span>
          </li>
        </ul>
        <button type="button" class="btn-primary" @click="router.push('/settings')">
          Open Settings
        </button>
      </div>
    </div>

    <template v-else>
      <div class="chat__body">
        <aside class="chat__rail" aria-label="Chat sessions">
          <div class="chat__rail-head">
            <span class="chat__rail-heading">Chats</span>
          </div>
          <button
            type="button"
            class="chat__rail-new"
            @click="startNewChat"
          >
            <Plus :size="14" />
            New chat
          </button>
          <div class="chat__rail-list">
            <div
              v-for="s in sessions"
              :key="s.id"
              class="chat__rail-row"
              :class="{ 'is-active': s.id === activeSessionId }"
              v-memo="[
                s.id === activeSessionId,
                s.title,
                s.updatedAt,
                renamingId === s.id,
                renamingId === s.id ? renameValue : '',
              ]"
            >
              <button type="button" class="chat__rail-item" @click="switchToSession(s.id)">
                <input
                  v-if="renamingId === s.id"
                  v-model="renameValue"
                  class="chat__rail-rename"
                  aria-label="Rename chat"
                  autofocus
                  @click.stop
                  @keydown.enter.prevent="commitRename"
                  @keydown.escape.prevent="cancelRename"
                  @blur="commitRename"
                />
                <span v-else class="chat__rail-item-title">{{ s.title }}</span>
                <span class="chat__rail-item-time">{{ relativeTime(s.updatedAt) }}</span>
              </button>
              <div class="chat__rail-actions">
                <button
                  type="button"
                  class="chat__rail-action"
                  title="Rename chat"
                  aria-label="Rename chat"
                  @click.stop="beginRename(s)"
                >
                  <Pencil :size="12" />
                </button>
                <button
                  type="button"
                  class="chat__rail-action chat__rail-action--danger"
                  title="Delete chat"
                  aria-label="Delete chat"
                  @click.stop="removeSession(s.id)"
                >
                  <Trash2 :size="12" />
                </button>
              </div>
            </div>
          </div>
        </aside>

        <div class="chat__main">
          <div class="chat__thread-bar">
            <span class="chat__thread-meta" :title="projectLabel">
              {{ projectLabel }}
            </span>
            <button
              v-if="activeMessages.length > 0"
              type="button"
              class="chat__clear"
              title="Clear this chat"
              @click="clearActiveChat"
            >
              <Eraser :size="13" />
              Clear
            </button>
          </div>

          <Transition name="chat-switch" mode="out-in">
            <div :key="activeSessionId ?? 'none'" class="chat__thread-wrap">
              <div v-if="activeMessages.length === 0" class="chat__thread-empty">
                <Sparkles :size="20" />
                <p class="chat__thread-empty-title">Start a conversation</p>
                <p class="chat__thread-empty-body">
                  Ask about the workspace, or pick a starter below.
                </p>
              </div>
              <div v-else ref="scroller" class="chat__thread">
                <div class="chat__thread-inner">
                  <!-- Plain list (no TransitionGroup) — avoids O(n) move FLIP
                       work when appending; v-memo keeps old markdown bubbles cold. -->
                  <div class="chat__thread-msgs">
                    <div
                      v-for="(m, mi) in activeMessages"
                      :key="m.id"
                      class="msg"
                      :class="`msg--${m.role}`"
                    >
                      <article
                        class="bubble"
                        :class="`bubble--${m.role}`"
                        v-memo="[
                          m.id,
                          m.text,
                          m.toolCalls,
                          isToolsHelpMessage(m) ? isToolsExpanded(m.id) : false,
                        ]"
                      >
                        <div v-if="m.toolCalls?.length" class="bubble__tools">
                          <div
                            v-for="(t, i) in m.toolCalls"
                            :key="`${m.id}-${t.toolId}-${i}`"
                            class="tool"
                            :class="t.ok ? 'tool--ok' : 'tool--fail'"
                          >
                            <Wrench :size="12" />
                            <span>{{ t.title }}</span>
                            <span v-if="!t.ok" class="tool__flag">fail</span>
                          </div>
                        </div>
                        <div v-if="isToolsHelpMessage(m)" class="tools-fold">
                          <button
                            type="button"
                            class="tools-fold__toggle"
                            :aria-expanded="isToolsExpanded(m.id)"
                            @click="toggleToolsExpanded(m.id)"
                          >
                            <ChevronDown
                              v-if="isToolsExpanded(m.id)"
                              :size="14"
                            />
                            <ChevronRight v-else :size="14" />
                            <span>{{ toolsHelpSummary(m.text) }}</span>
                            <span class="tools-fold__action">
                              {{ isToolsExpanded(m.id) ? "Collapse" : "Expand" }}
                            </span>
                          </button>
                          <ChatMessageContent
                            v-if="isToolsExpanded(m.id)"
                            :text="m.text"
                          />
                        </div>
                        <ChatMessageContent v-else :text="m.text" />
                        <template
                          v-if="
                            m.role === 'assistant' &&
                            currentProjectPath &&
                            patchIdsForMessage(m).length
                          "
                        >
                          <PatchApprovalCard
                            v-for="pid in patchIdsForMessage(m)"
                            :key="`${m.id}-${pid}`"
                            :project-path="currentProjectPath"
                            :patch-id="pid"
                            @verify="verifyFromPatch"
                          />
                        </template>
                      </article>
                      <div
                        v-if="showMessageActions(m)"
                        class="msg__actions"
                      >
                        <button
                          type="button"
                          class="msg__action"
                          title="Copy message"
                          aria-label="Copy message"
                          @click="copyMessage(m)"
                        >
                          <Check v-if="copiedMessageId === m.id" :size="12" />
                          <Copy v-else :size="12" />
                        </button>
                        <button
                          type="button"
                          class="msg__action"
                          title="Fork chat from here"
                          aria-label="Fork chat from here"
                          @click="forkFromMessage(mi)"
                        >
                          <GitFork :size="12" />
                        </button>
                      </div>
                    </div>
                  </div>
                  <Transition name="think">
                    <ChatThinkingBubble
                      v-if="busy"
                      :label="busyLabel"
                      :detail="busyDetail"
                    />
                  </Transition>
                  <VerifyLogPanel
                    v-if="verifyRunKey"
                    :run-key="verifyRunKey"
                    :active="busy"
                    :project-path="currentProjectPath || undefined"
                  />
                </div>
              </div>
            </div>
          </Transition>

          <div class="chat__modes" aria-label="Agent mode">
            <button
              v-for="m in (['ask', 'plan', 'act', 'delegate'] as const)"
              :key="m"
              type="button"
              class="chat__mode"
              :class="{ 'chat__mode--on': chatMode === m }"
              :disabled="busy"
              @click="chatMode = m"
            >
              {{ m }}
            </button>
          </div>

          <div class="chat__starters" aria-label="Starter tools">
            <button
              v-for="q in starterChips"
              :key="q"
              type="button"
              class="chat__chip"
              :disabled="busy"
              @click="insertSlash(q)"
            >
              {{ q }}
            </button>
            <button
              type="button"
              class="chat__chip chat__chip--more"
              :disabled="busy"
              title="List all tools"
              @click="runToolsDump"
            >
              More…
            </button>
          </div>

          <ChatComposer
            ref="composer"
            :busy="busy"
            @send="send"
            @stop="stopTurn"
          />
          <p class="chat__hint">
            Mode {{ chatMode }} · slash = local · freeform = Agent Engine ·
            <button type="button" class="chat__link" @click="runToolsDump">
              Tools
            </button>
          </p>
        </div>
      </div>

      <div
        v-if="actionToast"
        class="chat__toast"
        role="status"
        aria-live="polite"
      >
        {{ actionToast }}
      </div>
    </template>
  </div>
</template>

<style scoped>
.chat {
  display: flex;
  flex-direction: column;
  margin: -1.25rem;
  height: calc(100vh - 96px);
  min-height: 420px;
  background: var(--background);
  color: var(--foreground);
  overflow: hidden;
}

.chat__sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}

.chat__thread-bar {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  flex-shrink: 0;
  min-height: 1.5rem;
}

.chat__thread-meta {
  min-width: 0;
  flex: 1;
  font-size: 0.72rem;
  color: var(--muted-foreground);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.chat__clear {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  margin-left: auto;
  border: 1px solid transparent;
  background: transparent;
  color: var(--muted-foreground);
  font-size: 0.72rem;
  font-weight: 500;
  padding: 0.3rem 0.5rem;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.12s ease, border-color 0.12s ease, color 0.12s ease;
}

.chat__clear:hover {
  background: var(--surface-2);
  border-color: var(--border);
  color: var(--foreground);
}

.chat__modes {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  margin-bottom: 0.45rem;
}
.chat__mode {
  border: 1px solid var(--border);
  background: transparent;
  color: var(--muted-foreground);
  border-radius: 999px;
  padding: 0.2rem 0.65rem;
  font-size: 0.72rem;
  text-transform: capitalize;
  cursor: pointer;
}
.chat__mode--on {
  color: var(--foreground);
  border-color: color-mix(in srgb, var(--accent-blue, #3d7eff) 55%, var(--border));
  background: color-mix(in srgb, var(--accent-blue, #3d7eff) 16%, transparent);
}
.chat__mode:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
.chat__starters {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  flex-shrink: 0;
}

.chat__chip {
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--muted-foreground);
  border-radius: 999px;
  padding: 0.25rem 0.65rem;
  font-size: 0.72rem;
  font-weight: 500;
  cursor: pointer;
  transition: background 0.12s ease, border-color 0.12s ease, color 0.12s ease;
}

.chat__chip:hover:not(:disabled) {
  background: var(--surface-hover);
  border-color: var(--border-strong);
  color: var(--foreground);
}

.chat__chip:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.chat__chip--more {
  border-style: dashed;
  color: var(--foreground);
}

.chat__viewport {
  flex: 1;
  min-height: 0;
  display: flex;
  padding: 1.25rem 1.5rem;
}

.chat__empty {
  margin: auto;
  text-align: center;
  max-width: 360px;
  color: var(--muted-foreground);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
}

.chat__gate {
  margin: auto;
  max-width: 480px;
  width: 100%;
  padding: 1.25rem 1.35rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.chat__gate-head {
  display: flex;
  gap: 0.75rem;
  align-items: flex-start;
  color: var(--muted-foreground);
}

.chat__gate-title {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
  color: var(--foreground);
  letter-spacing: -0.02em;
}

.chat__gate-body {
  margin: 0.3rem 0 0;
  font-size: 0.8rem;
  line-height: 1.45;
  color: var(--muted-foreground);
}

.chat__gate-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.chat__gate-item {
  display: grid;
  grid-template-columns: auto 1fr auto;
  gap: 0.65rem;
  align-items: center;
  padding: 0.65rem 0.75rem;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--surface-2);
}

.chat__gate-icon--ok {
  color: var(--accent-green);
}

.chat__gate-icon--warn {
  color: var(--accent-amber);
}

.chat__gate-label {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--foreground);
}

.chat__gate-hint {
  font-size: 0.7rem;
  color: var(--muted-foreground);
  margin-top: 0.1rem;
}

.chat__gate-badge {
  font-size: 0.65rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--muted-foreground);
}

.chat__gate-badge.is-ok {
  color: var(--accent-green);
}

.chat__gate-badge.is-warn {
  color: var(--accent-amber);
}

.chat__empty-title {
  margin: 0;
  color: var(--foreground);
  font-weight: 600;
}

.chat__empty-body {
  margin: 0;
  font-size: 0.875rem;
  line-height: 1.45;
}

/* ============================================================================
   TWO-COLUMN CHAT BODY — rail (app-style chat list) + main thread
   ============================================================================ */

.chat__body {
  flex: 1;
  min-height: 0;
  display: flex;
  overflow: hidden;
}

.chat__rail {
  width: 224px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--divider);
  background: var(--surface-1);
  overflow: hidden;
}

.chat__rail-head {
  display: flex;
  align-items: center;
  flex-shrink: 0;
  padding: 0.9rem 0.85rem 0.45rem;
}

.chat__rail-heading {
  font-size: 0.66rem;
  font-weight: 600;
  letter-spacing: 0.07em;
  text-transform: uppercase;
  color: var(--muted-foreground);
  opacity: 0.7;
}

.chat__rail-new {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
  margin: 0 0.5rem 0.55rem;
  padding: 0.45rem 0.65rem;
  border-radius: 9px;
  border: 1px solid var(--border-strong);
  background: var(--surface-3);
  color: var(--foreground);
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.12s ease, border-color 0.12s ease;
}

.chat__rail-new:hover {
  background: var(--surface-hover);
  border-color: var(--border-strong);
}

.chat__rail-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 0.15rem 0.5rem 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
}

.chat__rail-row {
  position: relative;
  display: flex;
  align-items: stretch;
  border-radius: 9px;
  transition: background 0.12s ease;
}

.chat__rail-row:hover {
  background: var(--surface-highlight);
}

.chat__rail-row.is-active {
  background: var(--sidebar-accent);
}

.chat__rail-row.is-active:hover {
  background: var(--sidebar-accent);
}

.chat__rail-item {
  flex: 1;
  min-width: 0;
  text-align: left;
  background: transparent;
  border: none;
  padding: 0.5rem 0.6rem;
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
  cursor: pointer;
  border-radius: 9px;
}

.chat__rail-item-title {
  font-size: 0.79rem;
  font-weight: 500;
  color: var(--foreground);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.chat__rail-row.is-active .chat__rail-item-title {
  font-weight: 600;
}

.chat__rail-item-time {
  font-size: 0.65rem;
  color: var(--muted-foreground);
  opacity: 0.75;
}

.chat__rail-rename {
  width: 100%;
  background: var(--surface-3);
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  padding: 0.15rem 0.35rem;
  font: inherit;
  font-size: 0.79rem;
  color: var(--foreground);
  outline: none;
}

.chat__rail-actions {
  display: flex;
  align-items: center;
  gap: 0.1rem;
  padding-right: 0.35rem;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.12s ease;
}

.chat__rail-row:hover .chat__rail-actions,
.chat__rail-row:focus-within .chat__rail-actions {
  opacity: 1;
  pointer-events: auto;
}

.chat__rail-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--muted-foreground);
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease;
}

.chat__rail-action:hover {
  background: var(--surface-hover);
  color: var(--foreground);
}

.chat__rail-action--danger:hover {
  color: var(--accent-red);
  background: rgba(239, 68, 68, 0.1);
}

.chat__main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  padding: 0.75rem 1.25rem 1rem;
  overflow: hidden;
}

.chat__thread-wrap {
  flex: 1;
  min-height: 0;
  display: flex;
}

.chat__thread-empty {
  margin: auto;
  text-align: center;
  max-width: 300px;
  color: var(--muted-foreground);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.4rem;
}

.chat__thread-empty-title {
  margin: 0;
  color: var(--foreground);
  font-weight: 600;
  font-size: 0.9rem;
}

.chat__thread-empty-body {
  margin: 0;
  font-size: 0.8rem;
  line-height: 1.4;
}

.chat__thread {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding-right: 0.25rem;
}

.chat__thread-inner {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.chat__thread-msgs {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.msg {
  display: flex;
  flex-direction: column;
  gap: 0.28rem;
  max-width: min(820px, 100%);
}

.msg--user {
  align-self: flex-end;
  align-items: flex-end;
}

.msg--assistant,
.msg--system {
  align-self: flex-start;
  align-items: flex-start;
}

.msg__actions {
  display: flex;
  align-items: center;
  gap: 0.1rem;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.12s ease;
}

.msg:hover .msg__actions,
.msg:focus-within .msg__actions {
  opacity: 1;
  pointer-events: auto;
}

.msg__action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--muted-foreground);
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease;
}

.msg__action:hover {
  background: var(--surface-hover);
  color: var(--foreground);
}

.chat__toast {
  position: fixed;
  bottom: 1.5rem;
  right: 1.5rem;
  z-index: 50;
  padding: 0.55rem 0.85rem;
  border-radius: 10px;
  border: 1px solid color-mix(in srgb, var(--accent-green) 30%, var(--border));
  background: color-mix(in srgb, var(--accent-green) 12%, var(--surface-2));
  color: var(--accent-green);
  font-size: 0.78rem;
  font-weight: 500;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.28);
  pointer-events: none;
}

.bubble {
  width: 100%;
  border-radius: 12px;
  padding: 0.75rem 0.9rem;
  border: 1px solid var(--border);
  background: var(--surface-2);
}

.bubble--user {
  background: var(--surface-3);
  border-color: var(--border-strong);
  transform-origin: bottom right;
}

.bubble--assistant,
.bubble--system {
  transform-origin: bottom left;
}

.bubble__tools {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
  margin-bottom: 0.5rem;
}

.tool {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  font-size: 0.68rem;
  padding: 0.2rem 0.5rem;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--surface-3);
  color: var(--muted-foreground);
}

.tool--ok {
  color: var(--accent-green);
  border-color: color-mix(in srgb, var(--accent-green) 35%, var(--border));
}

.tool--fail {
  color: var(--accent-red);
  border-color: color-mix(in srgb, var(--accent-red) 35%, var(--border));
}

.tool__flag {
  opacity: 0.7;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.tools-fold {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
}

.tools-fold__toggle {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  width: 100%;
  margin: 0;
  padding: 0.15rem 0;
  border: none;
  background: transparent;
  color: var(--foreground);
  font: inherit;
  font-size: 0.8rem;
  font-weight: 500;
  text-align: left;
  cursor: pointer;
}

.tools-fold__toggle:hover {
  color: var(--foreground);
}

.tools-fold__action {
  margin-left: auto;
  font-size: 0.68rem;
  font-weight: 500;
  color: var(--muted-foreground);
}

.chat__hint {
  margin: 0;
  font-size: 0.7rem;
  color: var(--muted-foreground);
  flex-shrink: 0;
  line-height: 1.35;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.chat__hint code {
  font-family: var(--font-mono);
  font-size: 0.68rem;
}

.chat__link {
  background: none;
  border: none;
  color: var(--foreground);
  cursor: pointer;
  padding: 0;
  font: inherit;
  text-decoration: underline;
  text-underline-offset: 2px;
  opacity: 0.85;
}

.chat__link:hover {
  opacity: 1;
}

/* ============================================================================
   MOTION — session switch, snappy message pop-in, thinking bubble enter
   All motion is in-chat only (never an OS wait cursor).
   ============================================================================ */

.chat-switch-enter-active,
.chat-switch-leave-active {
  transition: opacity 140ms ease, transform 140ms ease;
}

.chat-switch-enter-from,
.chat-switch-leave-to {
  opacity: 0;
  transform: translateY(3px);
}

/* User bubble: snappy pop / slide-up (premium, short) */
.msg-enter-active {
  transition:
    opacity 180ms cubic-bezier(0.22, 1, 0.36, 1),
    transform 180ms cubic-bezier(0.22, 1, 0.36, 1);
}

.msg-enter-from {
  opacity: 0;
  transform: translateY(10px) scale(0.97);
}

.bubble--user.msg-enter-active {
  transition:
    opacity 160ms cubic-bezier(0.22, 1, 0.36, 1),
    transform 160ms cubic-bezier(0.34, 1.4, 0.64, 1);
}

.bubble--user.msg-enter-from {
  opacity: 0;
  transform: translateY(12px) scale(0.94);
}

.msg-leave-active {
  transition: opacity 140ms ease;
}

.msg-leave-to {
  opacity: 0;
}

.msg-move {
  transition: transform 180ms ease;
}

/* Thinking bubble slides in right after the user message paints */
.think-enter-active {
  transition:
    opacity 160ms cubic-bezier(0.22, 1, 0.36, 1),
    transform 160ms cubic-bezier(0.22, 1, 0.36, 1);
}

.think-leave-active {
  transition: opacity 120ms ease, transform 120ms ease;
}

.think-enter-from {
  opacity: 0;
  transform: translateY(8px) scale(0.98);
}

.think-leave-to {
  opacity: 0;
  transform: translateY(-2px);
}

@media (prefers-reduced-motion: reduce) {
  .chat-switch-enter-active,
  .chat-switch-leave-active,
  .msg-enter-active,
  .msg-leave-active,
  .msg-move,
  .bubble--user.msg-enter-active,
  .think-enter-active,
  .think-leave-active {
    transition: none !important;
  }

  .chat-switch-enter-from,
  .chat-switch-leave-to,
  .msg-enter-from,
  .msg-leave-to,
  .bubble--user.msg-enter-from,
  .think-enter-from,
  .think-leave-to {
    opacity: 1 !important;
    transform: none !important;
  }
}
</style>
