<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  RefreshCw,
  Search,
  FileText,
  FileEdit,
  Camera,
  ListTodo,
  Clock,
  Bot,
  FolderOpen,
  ShieldAlert,
  CheckCircle2,
  AlertCircle,
  Loader2,
  ChevronDown,
  X,
  ExternalLink,
} from "lucide-vue-next";
import { useStore } from "../lib/useStore";
import {
  type RefreshResult,
  type ContextSearchResult,
  type ContextInspection,
  type ContextHealth,
  refreshContext,
  searchContext,
  inspectContext,
  getContextHealth,
} from "../lib/contextClient";

const { currentProject, currentProjectPath } = useStore();
const route = useRoute();
const router = useRouter();

// --- State ---
const query = ref("");
const executedQuery = ref("");
const limit = ref(25);
const activeKinds = ref<Set<string>>(new Set());
const results = ref<ContextSearchResult[]>([]);
const selectedResult = ref<ContextSearchResult | null>(null);
const inspection = ref<ContextInspection | null>(null);
const refreshResult = ref<RefreshResult | null>(null);
const health = ref<ContextHealth | null>(null);
const loading = ref(false);
const searching = ref(false);
const refreshing = ref(false);
const error = ref<string | null>(null);
const expandedSections = ref<Set<string>>(new Set(["results"]));
const searchInputRef = ref<HTMLInputElement | null>(null);
const hasExecutedSearch = ref(false);

const DEFAULT_LIMIT = 25;
const MAX_LIMIT = 100;
const MAX_PREVIEW_CHARS = 4000;

const ALL_KINDS = [
  { value: "doc", label: "Docs", icon: FileText },
  { value: "note", label: "Notes", icon: FileEdit },
  { value: "snapshot", label: "Snapshots", icon: Camera },
  { value: "task", label: "Tasks", icon: ListTodo },
  { value: "recent", label: "Recent", icon: Clock },
  { value: "agent-session", label: "Agent Sessions", icon: Bot },
];

const hasProject = computed(() => !!currentProjectPath.value);
const hasProjectObj = computed(() => !!currentProject.value);
const indexedDocumentCount = computed(() => health.value?.document_count ?? 0);
const isHealthy = computed(() => health.value?.integrity_ok && health.value.document_count > 0);
const isDegraded = computed(() => health.value && !health.value.integrity_ok);
const isEmpty = computed(() => health.value?.document_count === 0);
const noIndex = computed(
  () =>
    !!currentProjectPath.value &&
    health.value &&
    health.value.document_count === 0 &&
    !currentProject.value
);

const refreshStatusLabel = computed(() => {
  if (!refreshResult.value) return null;
  return refreshResult.value.status;
});

const refreshStatusClass = computed(() => {
  const s = refreshResult.value?.status;
  if (s === "COMPLETE") return "badge-success";
  if (s === "PARTIAL") return "badge-warning";
  return "badge-danger";
});

function statusIcon(status: string) {
  if (status === "COMPLETE") return CheckCircle2;
  if (status === "PARTIAL") return AlertCircle;
  return AlertCircle;
}

function iconForKind(kind: string) {
  return ALL_KINDS.find((k) => k.value === kind)?.icon ?? FileText;
}

function labelForKind(kind: string) {
  return ALL_KINDS.find((k) => k.value === kind)?.label ?? kind;
}

function toggleKind(kind: string) {
  const next = new Set(activeKinds.value);
  if (next.has(kind)) next.delete(kind);
  else next.add(kind);
  activeKinds.value = next;
  if (query.value.trim()) runSearch();
}

async function runSearch() {
  if (!currentProjectPath.value) { error.value = "No project selected."; return; }
  if (!query.value.trim()) { results.value = []; return; }
  searching.value = true;
  error.value = null;
  try {
    const kinds = activeKinds.value.size > 0 ? [...activeKinds.value] : undefined;
    results.value = await searchContext(currentProjectPath.value, query.value, { kinds, limit: limit.value });
    executedQuery.value = query.value;
    hasExecutedSearch.value = true;
  } catch (e: any) {
    error.value = `Search failed: ${String(e)}`;
  } finally {
    searching.value = false;
  }
}

async function openInspector(result: ContextSearchResult) {
  selectedResult.value = result;
  inspection.value = null;
  try {
    inspection.value = await inspectContext(currentProjectPath.value!, result.document_id);
  } catch (e: any) {
    error.value = `Inspection failed: ${String(e)}`;
  }
}

async function doRefresh() {
  if (!currentProjectPath.value) { error.value = "No project selected."; return; }
  refreshing.value = true;
  error.value = null;
  refreshResult.value = null;
  try {
    const r = await refreshContext(currentProjectPath.value);
    refreshResult.value = r;
    await loadHealth();
    if (query.value.trim()) runSearch();
  } catch (e: any) {
    error.value = `Refresh failed: ${String(e)}`;
  } finally {
    refreshing.value = false;
  }
}

async function loadHealth() {
  if (!currentProjectPath.value) { health.value = null; return; }
  try {
    health.value = await getContextHealth(currentProjectPath.value);
  } catch { health.value = null; }
}

function previewText(doc: ContextInspection | null): string {
  if (!doc) return "";
  if (doc.sensitivity === "secret") return "[secret content hidden]";
  if (doc.text.length > MAX_PREVIEW_CHARS) return doc.text.slice(0, MAX_PREVIEW_CHARS) + "…";
  return doc.text;
}

/**
 * Parse a canonical_ref and return its components.
 * Format: openmesh://project/{projectId}/{kind}/{sourceKey}
 */
function parseCanonicalRef(ref: string): { projectId: string; kind: string; sourceKey: string } | null {
  try {
    if (!ref.startsWith("openmesh://project/")) return null;
    const rest = ref.slice("openmesh://project/".length);
    const parts = rest.split("/");
    if (parts.length < 3) return null;
    const projectId = decodeURIComponent(parts[0]);
    const kind = decodeURIComponent(parts[1]);
    const sourceKey = parts.slice(2).join("/");
    return { projectId, kind, sourceKey };
  } catch {
    return null;
  }
}

/**
 * Open the source document for the selected context result.
 * Navigates to the appropriate page in the current workspace.
 */
function openSource(): void {
  if (!selectedResult.value) return;
  const parsed = parseCanonicalRef(selectedResult.value.canonical_ref);
  if (!parsed) {
    error.value = "Invalid canonical reference.";
    return;
  }
  // Project scope check: must match current project.
  if (currentProject.value && parsed.projectId !== currentProject.value.id) {
    error.value = "Cannot open source from another project.";
    return;
  }
  switch (parsed.kind) {
    case "doc":
      router.push({ path: "/docs", query: { file: parsed.sourceKey } });
      break;
    case "note":
      router.push({ path: "/notes", query: { file: parsed.sourceKey } });
      break;
    case "snapshot":
      // Snapshots live in notes/snapshots/ folder; navigate to notes.
      router.push({ path: "/notes", query: { file: parsed.sourceKey } });
      break;
    case "task":
      router.push({ path: "/sprint", query: { task: parsed.sourceKey } });
      break;
    case "agent-session":
      router.push({ path: "/agent-sessions", query: { session: parsed.sourceKey } });
      break;
    case "recent":
      // No dedicated page for RecentItem source opening.
      error.value = "Open Source not supported for recent items.";
      break;
    default:
      error.value = `Open Source not supported for kind: ${parsed.kind}`;
  }
}

/**
 * Whether Open Source is available for the selected result.
 * Currently only supports doc and note kinds with full deep-link support.
 */
const canOpenSource = computed(() => {
  if (!selectedResult.value) return false;
  const parsed = parseCanonicalRef(selectedResult.value.canonical_ref);
  if (!parsed) return false;
  // Only doc and note have full deep-link support in destination pages
  return ["doc", "note"].includes(parsed.kind);
});

onMounted(async () => {
  await loadHealth();
  // Handle initial focus from Command Palette navigation
  if (route.query.focus === "search") {
    await nextTick();
    // Use setTimeout to ensure DOM is fully ready
    setTimeout(() => {
      searchInputRef.value?.focus();
    }, 50);
  }
});
watch(currentProjectPath, () => { loadHealth(); selectedResult.value = null; inspection.value = null; results.value = []; hasExecutedSearch.value = false; });

// Focus search input when route changes to focus=search
watch(
  () => route.query.focus,
  async (focus) => {
    if (focus === "search") {
      await nextTick();
      setTimeout(() => {
        searchInputRef.value?.focus();
      }, 50);
    }
  },
);

// Optional: preserve initial query from Command Palette
watch(
  () => route.query.q,
  (q) => {
    if (typeof q === "string" && q.length > 0 && query.value === "") {
      query.value = q;
    }
  },
  { immediate: true },
);
</script>

<template>
  <div class="h-full flex flex-col min-w-0 overflow-hidden">
    <!-- Header -->
    <div class="px-5 py-3 flex items-center gap-3 flex-shrink-0" style="border-bottom: 1px solid var(--divider)">
      <div class="flex items-center gap-2 flex-1 min-w-0">
        <h1 class="text-[14px] font-semibold truncate" style="color: var(--foreground)">Context Search</h1>
        <span v-if="hasProject" class="text-[11px] truncate" style="color: var(--muted-foreground)">
          {{ currentProject?.name }}
        </span>
      </div>
      <button
        class="btn-sm"
        :disabled="refreshing || !hasProject"
        @click="doRefresh"
        title="Refresh Context"
      >
        <Loader2 v-if="refreshing" class="h-3 w-3 animate-spin" />
        <RefreshCw v-else class="h-3 w-3" />
        <span class="text-[11px]">Refresh Context</span>
      </button>
    </div>

    <!-- No Project -->
    <div v-if="!hasProject" class="flex-1 flex items-center justify-center p-8">
      <div class="text-center text-[12px] max-w-xs" style="color: var(--muted-foreground)">
        <FolderOpen class="h-8 w-8 mx-auto mb-3 opacity-40" />
        <p class="font-medium mb-1">No project selected</p>
        <p>Select a project to search its context.</p>
      </div>
    </div>

    <!-- Main Content -->
    <div v-else class="flex-1 flex flex-col min-w-0 overflow-hidden">
      <!-- Search Bar -->
      <div class="px-5 py-3 flex-shrink-0" style="border-bottom: 1px solid var(--divider)">
        <div class="flex items-center gap-2" style="background: var(--surface-highlight); border: 1px solid var(--border); border-radius: 6px; padding: 6px 10px;">
          <Search class="h-3.5 w-3.5 flex-shrink-0" style="color: var(--muted-foreground)" />
          <input
            ref="searchInputRef"
            v-model="query"
            type="text"
            placeholder="Search your context…"
            class="flex-1 bg-transparent border-none outline-none text-[12px]"
            style="color: var(--foreground)"
            @keyup.enter="runSearch"
          />
          <button v-if="query" class="p-0.5" @click="query = ''; runSearch()" title="Clear">
            <X class="h-3 w-3" style="color: var(--muted-foreground)" />
          </button>
        </div>

        <!-- Kind Filters -->
        <div class="flex items-center gap-1.5 mt-2 flex-wrap">
          <button
            v-for="k in ALL_KINDS"
            :key="k.value"
            class="badge"
            :class="{ 'badge-active': activeKinds.has(k.value) }"
            @click="toggleKind(k.value)"
          >
            <component :is="k.icon" class="h-2.5 w-2.5" />
            <span>{{ k.label }}</span>
          </button>
          <div class="flex-1" />
          <span class="text-[10px]" style="color: var(--muted-foreground)">Limit</span>
          <select v-model.number="limit" class="input-xs" style="width: auto">
            <option :value="10">10</option>
            <option :value="25">25</option>
            <option :value="50">50</option>
            <option :value="MAX_LIMIT">{{ MAX_LIMIT }}</option>
          </select>
        </div>

        <!-- Health / Status -->
        <div class="flex items-center gap-3 mt-2 text-[10px]" style="color: var(--muted-foreground)">
          <span class="flex items-center gap-1">
            <component :is="isHealthy ? CheckCircle2 : AlertCircle" class="h-3 w-3" />
            <template v-if="isHealthy">Healthy — {{ indexedDocumentCount }} docs indexed</template>
            <template v-else-if="isDegraded">Degraded</template>
            <template v-else-if="isEmpty">Empty — no context indexed yet</template>
            <template v-else>Loading health…</template>
          </span>
          <span v-if="refreshResult" class="flex items-center gap-1">
            <component :is="statusIcon(refreshResult.status)" class="h-3 w-3" />
            <span :class="refreshStatusClass">{{ refreshStatusLabel }}</span>
          </span>
        </div>
      </div>

      <!-- Error -->
      <div v-if="error" class="px-5 py-2 flex-shrink-0 text-[11px] bg-red-500/10 border-y border-red-500/20 text-red-400">
        {{ error }}
      </div>

      <!-- Refresh Summary -->
      <div v-if="refreshResult" class="px-5 py-3 flex-shrink-0 text-[11px] grid grid-cols-4 gap-2" style="border-bottom: 1px solid var(--divider)">
        <div><span class="font-medium text-green-400">{{ refreshResult.indexed }}</span> indexed</div>
        <div><span class="font-medium text-blue-400">{{ refreshResult.updated }}</span> updated</div>
        <div><span class="font-medium">{{ refreshResult.unchanged }}</span> unchanged</div>
        <div><span class="font-medium text-amber-400">{{ refreshResult.failed }}</span> failed</div>
      </div>

      <!-- Body -->
      <div class="flex-1 flex min-w-0 overflow-hidden">
        <!-- Results -->
        <div class="flex-1 flex flex-col min-w-0 overflow-hidden" :class="{ 'w-1/2': selectedResult }">
          <!-- Empty States -->
          <div v-if="loading || searching" class="flex-1 flex items-center justify-center">
            <Loader2 class="h-6 w-6 animate-spin" style="color: var(--muted-foreground)" />
          </div>
          <div v-else-if="!query.trim() && results.length === 0" class="flex-1 flex items-center justify-center p-8">
            <div class="text-center text-[12px]" style="color: var(--muted-foreground)">
              <Search class="h-8 w-8 mx-auto mb-3 opacity-40" />
              <p class="font-medium mb-1">Enter a search query</p>
              <p class="text-[11px]">your context across docs, notes, snapshots, tasks, sessions, and recent work</p>
            </div>
          </div>
          <div v-else-if="hasExecutedSearch && results.length === 0 && query === executedQuery" class="flex-1 flex items-center justify-center p-8 text-[12px]" style="color: var(--muted-foreground)">
            <div class="text-center">
              <AlertCircle class="h-8 w-8 mx-auto mb-3 opacity-40" />
              <p>No results for "{{ executedQuery }}"</p>
            </div>
          </div>
          <div v-else-if="query.trim() && (!hasExecutedSearch || query !== executedQuery) && results.length === 0" class="flex-1 flex items-center justify-center p-8 text-[12px]" style="color: var(--muted-foreground)">
            <div class="text-center">
              <Search class="h-8 w-8 mx-auto mb-3 opacity-40" />
              <p class="font-medium mb-1">Press Enter to search</p>
              <p class="text-[11px]">searching for "{{ query }}"</p>
            </div>
          </div>
          <!-- Result List -->
          <div v-else class="flex-1 overflow-y-auto">
            <div
              v-for="r in results"
              :key="r.document_id"
              class="px-5 py-3 cursor-pointer border-b transition-colors"
              style="border-color: var(--divider)"
              :class="{ 'bg-[var(--sidebar-accent)]': selectedResult?.document_id === r.document_id }"
              @click="openInspector(r)"
            >
              <div class="flex items-center gap-2 mb-1">
                <component :is="iconForKind(r.source_kind)" class="h-3 w-3 flex-shrink-0" style="color: var(--muted-foreground)" />
                <span class="text-[12px] font-medium truncate" style="color: var(--foreground)">{{ r.title }}</span>
                <span class="text-[10px] flex-shrink-0" style="color: var(--muted-foreground)">{{ labelForKind(r.source_kind) }}</span>
              </div>
              <p v-if="r.snippet" class="text-[11px] truncate" style="color: var(--muted-foreground)">{{ r.snippet }}</p>
            </div>
          </div>
        </div>

        <!-- Inspector -->
        <div v-if="selectedResult" class="border-l flex-shrink-0 overflow-y-auto" style="border-color: var(--divider); width: 40%; min-width: 240px">
          <div class="sticky top-0 px-4 py-3 flex items-center gap-2" style="background: var(--surface); border-bottom: 1px solid var(--divider)">
            <component :is="iconForKind(selectedResult.source_kind)" class="h-3.5 w-3.5" style="color: var(--muted-foreground)" />
            <span class="text-[12px] font-medium truncate flex-1" style="color: var(--foreground)">{{ selectedResult.title }}</span>
            <button class="p-0.5" @click="selectedResult = null">
              <X class="h-3 w-3" style="color: var(--muted-foreground)" />
            </button>
          </div>
          <div class="p-4 text-[11px] space-y-4">
            <!-- Metadata -->
            <div class="space-y-1.5">
              <div class="flex items-center gap-1.5">
                <span class="w-20" style="color: var(--muted-foreground)">Kind</span>
                <span>{{ labelForKind(selectedResult.source_kind) }}</span>
              </div>
              <div class="flex items-center gap-1.5">
                <span class="w-20" style="color: var(--muted-foreground)">Project</span>
                <span>{{ selectedResult.project_id }}</span>
              </div>
              <div v-if="inspection" class="flex items-center gap-1.5">
                <span class="w-20" style="color: var(--muted-foreground)">Sensitivity</span>
                <span :class="{ 'text-amber-400': inspection.sensitivity === 'secret' }">{{ inspection.sensitivity }}</span>
              </div>
              <div v-if="inspection" class="flex items-center gap-1.5">
                <span class="w-20" style="color: var(--muted-foreground)">Agent Context</span>
                <span>{{ inspection.agent_context_enabled ? "Enabled" : "Off" }}</span>
              </div>
              <div class="flex items-center gap-1.5">
                <span class="w-20" style="color: var(--muted-foreground)">Canonical</span>
                <span class="truncate font-mono text-[10px]">{{ selectedResult.canonical_ref }}</span>
              </div>
              <div v-if="inspection" class="flex items-center gap-1.5">
                <span class="w-20" style="color: var(--muted-foreground)">Observed</span>
                <span>{{ inspection.observed_at }}</span>
              </div>
              <div v-if="inspection?.source_updated_at" class="flex items-center gap-1.5">
                <span class="w-20" style="color: var(--muted-foreground)">Source Updated</span>
                <span>{{ inspection.source_updated_at }}</span>
              </div>
            </div>

            <hr style="border-color: var(--divider)" />

            <!-- Open Source -->
            <div v-if="canOpenSource" class="mt-3">
              <button class="btn-sm w-full justify-center" @click="openSource">
                <ExternalLink class="h-3 w-3" />
                <span class="text-[11px]">Open Source</span>
              </button>
            </div>

            <!-- Preview -->
            <div>
              <div class="flex items-center gap-1.5 mb-2">
                <span class="font-medium">Preview</span>
                <span class="text-[10px]" style="color: var(--muted-foreground)">(max {{ MAX_PREVIEW_CHARS }} chars)</span>
              </div>
              <pre class="whitespace-pre-wrap break-words text-[11px] p-3 rounded" style="background: var(--surface-highlight); color: var(--muted-foreground); font-family: inherit;">{{ previewText(inspection) }}</pre>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 9999px;
  font-size: 10px;
  border: 1px solid var(--border);
  color: var(--muted-foreground);
  background: transparent;
  cursor: pointer;
  transition: all 0.15s;
}
.badge:hover { background: var(--surface-highlight); }
.badge-active {
  background: var(--accent-blue, #7c3aed);
  color: white;
  border-color: var(--accent-blue, #7c3aed);
}
.btn-sm {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  border-radius: 4px;
  font-size: 11px;
  border: 1px solid var(--border);
  color: var(--foreground);
  background: var(--surface-highlight);
  cursor: pointer;
  transition: all 0.15s;
}
.btn-sm:hover:not(:disabled) { background: var(--sidebar-accent); }
.btn-sm:disabled { opacity: 0.5; cursor: not-allowed; }
.input-xs {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 2px 6px;
  font-size: 11px;
  color: var(--foreground);
}
.badge-success { color: #34d399; }
.badge-warning { color: #fbbf24; }
.badge-danger { color: #f87171; }
</style>
