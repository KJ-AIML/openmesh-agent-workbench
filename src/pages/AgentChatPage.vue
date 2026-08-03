<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  ref,
  shallowRef,
  triggerRef,
  watch,
} from "vue";
import { useRouter } from "vue-router";
import {
  AlertCircle,
  CheckCircle2,
  Eraser,
  MessageSquare,
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
import { resolveToolsForMessage } from "../lib/agentChat/tools";
import {
  chatModelId,
  getChatSetupChecks,
  isChatProviderReady,
} from "../lib/agentChat/ready";
import {
  type ChatMessage,
  type ChatSession,
  createChatMessage,
  createChatSession,
  deriveTitleFromMessage,
  loadSessions,
  persistSessions,
  touchSession,
} from "../lib/agentChat/chatSessions";
import { createPersistQueue } from "../lib/agentChat/persistQueue";
import ChatMessageContent from "../components/chat/ChatMessageContent.vue";
import ChatComposer from "../components/chat/ChatComposer.vue";
import ChatThinkingBubble from "../components/chat/ChatThinkingBubble.vue";

const router = useRouter();
const { currentProjectPath, currentProject, settings } = useStore();

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
const scroller = ref<HTMLElement | null>(null);
const composer = ref<InstanceType<typeof ChatComposer> | null>(null);

/** Rotating CLI-style status while waiting on the LLM (no streaming events). */
const LLM_STATUS_LINES = [
  "Thinking…",
  "Reasoning…",
  "Working with tools…",
  "Composing reply…",
] as const;
let statusRotateTimer: ReturnType<typeof setInterval> | null = null;
let statusRotateIdx = 0;

const persistQueue = createPersistQueue(persistSessions, 400);

const hasProject = computed(() => !!currentProjectPath.value);
const projectLabel = computed(
  () => currentProject.value?.name || currentProjectPath.value || "No project",
);
const chatReady = computed(() => isChatProviderReady(settings.value));
const setupChecks = computed(() => getChatSetupChecks(settings.value));
const activeModel = computed(() => chatModelId(settings.value));

const activeSession = computed(
  () => sessions.value.find((s) => s.id === activeSessionId.value) ?? null,
);

function welcomeText(): string {
  const provider = settings.value.provider?.name?.trim() || "provider";
  return (
    `OpenMesh Agent Engine · ${projectLabel.value}\n` +
    `Provider: ${provider} · Model: ${activeModel.value}\n\n` +
    "Slash tools run locally. Freeform messages use the live LLM tool loop.\n\n" +
    "Try /tools, /pilot, or ask “what’s in docs?”."
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

function loadForProject(path: string) {
  const loaded = loadSessions(path);
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
    if (path && ready) loadForProject(path);
    else {
      sessions.value = [];
      activeSessionId.value = null;
      activeMessages.value = [];
    }
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  stopStatusRotate();
  persistQueue.flush();
});

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
  if (session.titleIsDefault) {
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
    });
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
    touchSession(session);
    afterSessionMutation(session);
    await scrollBottom();
  }
}

function insertSlash(cmd: string) {
  composer.value?.insertSlash(cmd);
}

const quick = [
  "/tools",
  "/pilot",
  "/rc",
  "/team",
  "/search",
  "/git",
  "/pending",
  "/docs",
];
</script>

<template>
  <div class="chat">
    <header class="chat__head">
      <div class="chat__title-row">
        <MessageSquare class="chat__icon" :size="18" />
        <div>
          <h1 class="chat__title">Chat</h1>
          <p class="chat__sub">
            Tools bound to
            <span class="chat__path">{{ projectLabel }}</span>
          </p>
        </div>
        <button
          v-if="hasProject && chatReady && activeMessages.length > 0"
          type="button"
          class="chat__clear"
          title="Clear this chat"
          @click="clearActiveChat"
        >
          <Eraser :size="13" />
          Clear
        </button>
      </div>
      <div v-if="hasProject" class="chat__quick">
        <button
          v-for="q in quick"
          :key="q"
          type="button"
          class="chat__chip"
          @click="insertSlash(q)"
        >
          {{ q }}
        </button>
      </div>
    </header>

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
        <aside class="chat__rail">
          <div class="chat__rail-head">
            <span class="chat__rail-heading">Chats</span>
            <button
              type="button"
              class="chat__rail-new"
              title="New chat"
              @click="startNewChat"
            >
              <Plus :size="14" />
            </button>
          </div>
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
                  @click.stop="beginRename(s)"
                >
                  <Pencil :size="12" />
                </button>
                <button
                  type="button"
                  class="chat__rail-action chat__rail-action--danger"
                  title="Delete chat"
                  @click.stop="removeSession(s.id)"
                >
                  <Trash2 :size="12" />
                </button>
              </div>
            </div>
          </div>
        </aside>

        <div class="chat__main">
          <Transition name="chat-switch" mode="out-in">
            <div :key="activeSessionId ?? 'none'" class="chat__thread-wrap">
              <div v-if="activeMessages.length === 0" class="chat__thread-empty">
                <Sparkles :size="20" />
                <p class="chat__thread-empty-title">This chat is empty</p>
                <p class="chat__thread-empty-body">
                  Ask about the workspace, or try one of the tools above.
                </p>
              </div>
              <div v-else ref="scroller" class="chat__thread">
                <div class="chat__thread-inner">
                  <!-- Plain list (no TransitionGroup) — avoids O(n) move FLIP
                       work when appending; v-memo keeps old markdown bubbles cold. -->
                  <div class="chat__thread-msgs">
                    <article
                      v-for="m in activeMessages"
                      :key="m.id"
                      class="bubble"
                      :class="`bubble--${m.role}`"
                      v-memo="[m.id, m.text, m.toolCalls]"
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
                      <ChatMessageContent :text="m.text" />
                    </article>
                  </div>
                  <Transition name="think">
                    <ChatThinkingBubble
                      v-if="busy"
                      :label="busyLabel"
                      :detail="busyDetail"
                    />
                  </Transition>
                </div>
              </div>
            </div>
          </Transition>

          <ChatComposer ref="composer" :busy="busy" @send="send" />
          <p class="chat__hint">
            Slash tools run locally. Freeform uses OpenMesh Agent Engine (live LLM + tools).
            Needs a normal OpenAI-compatible provider — not DashScope Coding Plan.
            <button type="button" class="chat__link" @click="insertSlash('/tools')">
              Show tools
            </button>
          </p>
        </div>
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

.chat__head {
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  flex-shrink: 0;
  padding: 1.25rem 1.5rem 0.85rem;
}

.chat__title-row {
  display: flex;
  gap: 0.65rem;
  align-items: flex-start;
}

.chat__icon {
  margin-top: 0.2rem;
  color: var(--muted-foreground);
}

.chat__title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 600;
  letter-spacing: -0.02em;
  line-height: 1.2;
}

.chat__sub {
  margin: 0.15rem 0 0;
  font-size: 0.78rem;
  color: var(--muted-foreground);
}

.chat__path {
  color: var(--foreground);
  font-weight: 500;
}

.chat__clear {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  margin-left: auto;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--muted-foreground);
  font-size: 0.72rem;
  font-weight: 500;
  padding: 0.35rem 0.6rem;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.12s ease, border-color 0.12s ease, color 0.12s ease;
}

.chat__clear:hover {
  background: var(--surface-hover);
  color: var(--foreground);
  border-color: var(--border-strong);
}

.chat__quick {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
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

.chat__chip:hover {
  background: var(--surface-hover);
  border-color: var(--border-strong);
  color: var(--foreground);
}

.chat__viewport {
  flex: 1;
  min-height: 0;
  display: flex;
  padding: 0 1.5rem 1.25rem;
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
  justify-content: space-between;
  flex-shrink: 0;
  padding: 0.9rem 0.85rem 0.6rem;
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
  width: 24px;
  height: 24px;
  border-radius: 7px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--muted-foreground);
  cursor: pointer;
  transition: background 0.12s ease, border-color 0.12s ease, color 0.12s ease;
}

.chat__rail-new:hover {
  background: var(--surface-hover);
  color: var(--foreground);
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
  transition: opacity 0.12s ease;
}

.chat__rail-row:hover .chat__rail-actions,
.chat__rail-row.is-active .chat__rail-actions {
  opacity: 1;
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
  gap: 0.75rem;
  padding: 1rem 1.5rem 1.25rem;
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

.bubble {
  max-width: min(820px, 100%);
  border-radius: 12px;
  padding: 0.75rem 0.9rem;
  border: 1px solid var(--border);
  background: var(--surface-2);
}

.bubble--user {
  align-self: flex-end;
  background: var(--surface-3);
  border-color: var(--border-strong);
  transform-origin: bottom right;
}

.bubble--assistant,
.bubble--system {
  align-self: flex-start;
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

.chat__hint {
  margin: 0;
  font-size: 0.72rem;
  color: var(--muted-foreground);
  flex-shrink: 0;
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
