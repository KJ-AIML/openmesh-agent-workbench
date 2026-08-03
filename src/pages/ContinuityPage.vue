<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import {
  RefreshCw,
  Inbox,
  Clock,
  Users,
  Share2,
  Cloud,
  Loader2,
  AlertCircle,
  CheckCircle2,
  ShieldAlert,
  Shield,
  Plug,
  Network,
  ClipboardCheck,
  Flag,
  Radio,
} from "lucide-vue-next";
import { useStore } from "../lib/useStore";
import {
  type ContinuityHubSummary,
  type PendingQuestionsView,
  type ReturnDigest,
  type MeshPeerRecord,
  type MeshEnvelopeSummary,
  type RelayAuditEvent,
  type OnlineProxyConfig,
  type OnlineProxyAnswer,
  type TeamWorkspaceView,
  type TeamTrustPolicyView,
  type ConnectorDescriptorView,
  type OrgGraphView,
  type PilotPackView,
  type RcPackView,
  type LanServeStatus,
  type LanPeerInfo,
  getContinuityHubSummary,
  getPendingQuestions,
  getReturnDigest,
  listMeshPeers,
  listMeshEnvelopes,
  queryMeshPeer,
  listRelayAudit,
  getOnlineProxyStatus,
  initOnlineProxy,
  askOnlineProxy,
  getTeamWorkspace,
  getTeamTrustPolicy,
  listConnectors,
  getOrgGraph,
  getPilotStatus,
  getRcStatus,
  lanServeStart,
  lanServeStop,
  lanServeStatus,
  lanDiscover,
  lanSendPackage,
  lanAskPeer,
  type MeshRemoteQueryAnswer,
} from "../lib/continuityClient";

type TabId = "pending" | "digest" | "mesh" | "relay" | "online-proxy" | "lan" | "team" | "trust" | "connectors" | "org" | "pilot" | "rc";
type GroupId = "you" | "team" | "mesh" | "gate";

const { currentProject, currentProjectPath } = useStore();

const tab = ref<TabId>("pending");
const group = ref<GroupId>("you");
const loading = ref(false);
const error = ref<string | null>(null);
const summary = ref<ContinuityHubSummary | null>(null);

const pending = ref<PendingQuestionsView | null>(null);
const digest = ref<ReturnDigest | null>(null);
const digestHours = ref(24);
const peers = ref<MeshPeerRecord[]>([]);
const envelopes = ref<MeshEnvelopeSummary[]>([]);
const audit = ref<RelayAuditEvent[]>([]);
const onlineConfig = ref<OnlineProxyConfig | null>(null);
const askQuestion = ref("");
const askTier = ref("standard");
const lastAnswer = ref<OnlineProxyAnswer | null>(null);
const meshPeer = ref("");
const meshQuestion = ref("");
const meshTier = ref("low-impact");
const meshAnswer = ref<MeshRemoteQueryAnswer | null>(null);
const lanStatus = ref<LanServeStatus | null>(null);
const lanPeers = ref<LanPeerInfo[]>([]);
const lanSendId = ref("");
const lanSendTo = ref("");
const lanAskTo = ref("");
const lanAskQuestion = ref("");
const lanAskTier = ref("low-impact");
const lanAskAnswer = ref<MeshRemoteQueryAnswer | null>(null);
const acting = ref(false);
const teamWs = ref<TeamWorkspaceView | null>(null);
const trustPolicy = ref<TeamTrustPolicyView | null>(null);
const connectors = ref<ConnectorDescriptorView[]>([]);
const orgGraph = ref<OrgGraphView | null>(null);
const pilotPack = ref<PilotPackView | null>(null);
const rcPack = ref<RcPackView | null>(null);

const hasProject = computed(() => !!currentProjectPath.value);

const groups: { id: GroupId; label: string; tabs: TabId[] }[] = [
  { id: "you", label: "You", tabs: ["pending", "digest"] },
  { id: "team", label: "Team", tabs: ["team", "trust", "connectors", "org"] },
  { id: "mesh", label: "Mesh", tabs: ["mesh", "relay", "online-proxy", "lan"] },
  { id: "gate", label: "Gate", tabs: ["pilot", "rc"] },
];

const tabMeta: Record<TabId, { label: string; icon: typeof Inbox }> = {
  pending: { label: "Pending", icon: Inbox },
  digest: { label: "Digest", icon: Clock },
  mesh: { label: "Peers", icon: Users },
  team: { label: "Workspace", icon: Users },
  trust: { label: "Trust", icon: Shield },
  connectors: { label: "Connectors", icon: Plug },
  org: { label: "Org", icon: Network },
  pilot: { label: "Pilot", icon: ClipboardCheck },
  rc: { label: "RC", icon: Flag },
  relay: { label: "Relay", icon: Share2 },
  "online-proxy": { label: "Proxy", icon: Cloud },
  lan: { label: "LAN", icon: Radio },
};

const visibleTabs = computed(() => {
  const g = groups.find((x) => x.id === group.value) ?? groups[0];
  return g.tabs.map((id) => ({ id, ...tabMeta[id] }));
});

const statusLine = computed(() => {
  if (!summary.value) return null;
  const s = summary.value;
  return [
    `${s.openPendingCount} pending`,
    `${s.peerCount} peers`,
    s.onlineProxyInitialized ? "proxy on" : "proxy off",
  ].join(" · ");
});

function selectGroup(id: GroupId) {
  group.value = id;
  const g = groups.find((x) => x.id === id);
  if (g && !g.tabs.includes(tab.value)) {
    tab.value = g.tabs[0];
  }
}

function selectTab(id: TabId) {
  tab.value = id;
  const owner = groups.find((g) => g.tabs.includes(id));
  if (owner) group.value = owner.id;
}

async function loadSummary() {
  if (!currentProjectPath.value) {
    summary.value = null;
    return;
  }
  try {
    summary.value = await getContinuityHubSummary(currentProjectPath.value);
  } catch {
    summary.value = null;
  }
}

async function loadTab() {
  if (!currentProjectPath.value) {
    error.value = "No project selected.";
    return;
  }
  loading.value = true;
  error.value = null;
  try {
    const path = currentProjectPath.value;
    switch (tab.value) {
      case "pending":
        pending.value = await getPendingQuestions(path);
        break;
      case "digest":
        digest.value = await getReturnDigest(path, digestHours.value);
        break;
      case "mesh":
        peers.value = await listMeshPeers(path);
        envelopes.value = await listMeshEnvelopes(path);
        break;
      case "relay":
        audit.value = await listRelayAudit(path);
        break;
      case "online-proxy":
        onlineConfig.value = await getOnlineProxyStatus(path);
        break;
      case "lan":
        lanStatus.value = await lanServeStatus(path);
        break;
      case "team":
        teamWs.value = await getTeamWorkspace(path);
        break;
      case "trust":
        trustPolicy.value = await getTeamTrustPolicy(path);
        break;
      case "connectors":
        connectors.value = await listConnectors(path);
        break;
      case "org":
        orgGraph.value = await getOrgGraph(path);
        break;
      case "pilot":
        pilotPack.value = await getPilotStatus(path);
        break;
      case "rc":
        rcPack.value = await getRcStatus(path);
        break;
    }
    await loadSummary();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function refresh() {
  await loadTab();
}

async function handleInitProxy() {
  if (!currentProjectPath.value) return;
  acting.value = true;
  error.value = null;
  try {
    onlineConfig.value = await initOnlineProxy(currentProjectPath.value, {
      ownerLabel: currentProject.value?.name || "local-operator",
      mode: "local-scaffold",
      useRelayReceived: true,
    });
    await loadSummary();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function handleAsk() {
  if (!currentProjectPath.value || !askQuestion.value.trim()) return;
  acting.value = true;
  error.value = null;
  lastAnswer.value = null;
  try {
    lastAnswer.value = await askOnlineProxy(
      currentProjectPath.value,
      askQuestion.value.trim(),
      { tier: askTier.value },
    );
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function handleLanStart() {
  if (!currentProjectPath.value) return;
  acting.value = true;
  error.value = null;
  try {
    lanStatus.value = await lanServeStart(currentProjectPath.value, {
      ownerLabel: currentProject.value?.name || "local-operator",
    });
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function handleLanStop() {
  acting.value = true;
  error.value = null;
  try {
    lanStatus.value = await lanServeStop();
    if (currentProjectPath.value) {
      lanStatus.value = await lanServeStatus(currentProjectPath.value);
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function handleLanDiscover() {
  if (!currentProjectPath.value) return;
  acting.value = true;
  error.value = null;
  try {
    lanPeers.value = await lanDiscover(currentProjectPath.value, { seconds: 3 });
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

function selectLanPeer(peer: LanPeerInfo) {
  lanSendTo.value = peer.address;
  lanAskTo.value = peer.address;
}

async function handleLanSend() {
  if (!currentProjectPath.value || !lanSendId.value.trim() || !lanSendTo.value.trim()) {
    return;
  }
  acting.value = true;
  error.value = null;
  try {
    await lanSendPackage(
      currentProjectPath.value,
      lanSendId.value.trim(),
      lanSendTo.value.trim(),
    );
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function handleLanAsk() {
  if (!lanAskTo.value.trim() || !lanAskQuestion.value.trim()) return;
  acting.value = true;
  error.value = null;
  lanAskAnswer.value = null;
  try {
    lanAskAnswer.value = await lanAskPeer(
      lanAskTo.value.trim(),
      lanAskQuestion.value.trim(),
      { tier: lanAskTier.value },
    );
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function handleMeshQuery() {
  if (!currentProjectPath.value || !meshPeer.value.trim() || !meshQuestion.value.trim()) {
    return;
  }
  acting.value = true;
  error.value = null;
  meshAnswer.value = null;
  try {
    meshAnswer.value = await queryMeshPeer(
      currentProjectPath.value,
      meshPeer.value.trim(),
      meshQuestion.value.trim(),
      { tier: meshTier.value },
    );
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

function missedCounts(d: ReturnDigest) {
  const m = d.whatIMissed;
  return {
    completed: m.completed?.length ?? 0,
    changed: m.changed?.length ?? 0,
    blocked: m.blocked?.length ?? 0,
    decided: m.decided?.length ?? 0,
    needsAttention: m.needsAttention?.length ?? 0,
    stillOpen: m.stillOpen?.length ?? 0,
  };
}

watch(tab, () => {
  const owner = groups.find((g) => g.tabs.includes(tab.value));
  if (owner) group.value = owner.id;
  loadTab();
});

watch(currentProjectPath, () => {
  pending.value = null;
  digest.value = null;
  peers.value = [];
  envelopes.value = [];
  audit.value = [];
  onlineConfig.value = null;
  lastAnswer.value = null;
  teamWs.value = null;
  trustPolicy.value = null;
  connectors.value = [];
  orgGraph.value = null;
  pilotPack.value = null;
  rcPack.value = null;
  loadTab();
});

onMounted(() => {
  loadTab();
});
</script>

<template>
  <div class="cont animate-fade-in">
    <header class="cont__head">
      <div class="cont__head-main">
        <h1 class="cont__title">Continuity</h1>
        <p v-if="hasProject && statusLine" class="cont__meta">{{ statusLine }}</p>
        <p v-else-if="!hasProject" class="cont__meta">Select a project to load surfaces</p>
      </div>
      <button
        type="button"
        class="cont__refresh"
        :disabled="!hasProject || loading"
        title="Refresh"
        @click="refresh"
      >
        <RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': loading }" />
      </button>
    </header>

    <div
      v-if="!hasProject"
      class="workbench-card p-8 text-center text-[13px] text-muted"
    >
      No project selected. Choose a project in the sidebar to view continuity.
    </div>

    <template v-else>
      <nav class="om-nav" aria-label="Continuity sections">
        <div class="om-seg" role="tablist" aria-label="Section groups">
          <button
            v-for="g in groups"
            :key="g.id"
            type="button"
            role="tab"
            class="om-seg__btn"
            :class="{ 'is-active': group === g.id }"
            :aria-selected="group === g.id"
            @click="selectGroup(g.id)"
          >
            {{ g.label }}
          </button>
        </div>
        <div class="om-tabs" role="tablist" aria-label="Views">
          <button
            v-for="t in visibleTabs"
            :key="t.id"
            type="button"
            role="tab"
            class="om-tab"
            :class="{ 'is-active': tab === t.id }"
            :aria-selected="tab === t.id"
            @click="selectTab(t.id)"
          >
            <component :is="t.icon" class="h-3.5 w-3.5" />
            {{ t.label }}
          </button>
        </div>
      </nav>

      <div
        v-if="error"
        class="workbench-card p-4 flex items-start gap-2 text-[13px]"
        style="border-color: rgba(239, 68, 68, 0.35)"
      >
        <AlertCircle class="h-4 w-4 flex-shrink-0 mt-0.5" style="color: var(--accent-red)" />
        <span>{{ error }}</span>
      </div>

      <div v-if="loading" class="flex items-center gap-2 text-[13px] text-muted py-8 justify-center">
        <Loader2 class="h-4 w-4 animate-spin" />
        Loading…
      </div>

      <!-- Pending -->
      <div v-else-if="tab === 'pending'" class="space-y-3">
        <div v-if="pending" class="workbench-card p-5 space-y-3">
          <div
            v-if="pending.items.length === 0"
            class="cont__empty"
          >
            <p class="cont__empty-title">You're clear</p>
            <p class="cont__empty-body">
              No pending questions
              <span class="text-muted">
                · proxy {{ pending.sourceCounts.proxyPending }}
                · attention {{ pending.sourceCounts.continuityAttention }}
                · signal {{ pending.sourceCounts.unresolvedSignal }}
              </span>
            </p>
          </div>
          <div
            v-else
            class="flex items-center justify-between text-[12px] text-muted"
          >
            <span>{{ pending.openCount }} open</span>
            <span>
              proxy {{ pending.sourceCounts.proxyPending }} · attention
              {{ pending.sourceCounts.continuityAttention }} · signal
              {{ pending.sourceCounts.unresolvedSignal }}
            </span>
          </div>
          <div
            v-for="item in pending.items"
            :key="item.id"
            class="rounded-lg p-3 space-y-1"
            style="background: var(--surface-2); border: 1px solid var(--border)"
          >
            <div class="flex items-center gap-2 text-[12px]">
              <span class="badge">{{ item.severity }}</span>
              <span class="text-muted">{{ item.source }}</span>
              <span class="text-muted">·</span>
              <span class="text-muted">{{ item.status }}</span>
            </div>
            <div class="text-[13px] font-medium">{{ item.summary }}</div>
            <div v-if="item.reason" class="text-[12px] text-muted">{{ item.reason }}</div>
          </div>
          <div
            v-for="lim in pending.limitations"
            :key="lim"
            class="text-[11px] text-muted"
          >
            limitation: {{ lim }}
          </div>
        </div>
      </div>

      <!-- Digest -->
      <div v-else-if="tab === 'digest'" class="space-y-3">
        <div class="flex items-center gap-2">
          <label class="text-[12px] text-muted">Window (hours)</label>
          <input
            v-model.number="digestHours"
            type="number"
            min="1"
            max="720"
            class="input-sm w-20"
            @change="loadTab"
          />
        </div>
        <div v-if="digest" class="workbench-card p-4 space-y-4">
          <p class="text-[13px]">{{ digest.summary }}</p>
          <div class="text-[11px] text-muted">
            {{ digest.window.since }} → {{ digest.window.until }}
          </div>
          <div>
            <h3 class="section-label">Needs me ({{ digest.needsMe.length }})</h3>
            <div v-if="digest.needsMe.length === 0" class="text-[12px] text-muted">
              (nothing pending)
            </div>
            <ul v-else class="space-y-1 mt-1">
              <li
                v-for="item in digest.needsMe"
                :key="item.id"
                class="text-[12px]"
              >
                [{{ item.severity }}] {{ item.summary }}
              </li>
            </ul>
          </div>
          <div>
            <h3 class="section-label">What I missed</h3>
            <p class="text-[12px] text-muted mt-1">{{ digest.catchUpSummary }}</p>
            <div class="grid grid-cols-2 sm:grid-cols-3 gap-2 mt-2 text-[11px]">
              <span
                v-for="(n, k) in missedCounts(digest)"
                :key="k"
                class="chip"
              >{{ k }}: {{ n }}</span>
            </div>
          </div>
          <div>
            <h3 class="section-label">Handoffs ({{ digest.handoffs.length }})</h3>
            <div v-if="digest.handoffs.length === 0" class="text-[12px] text-muted">
              (no handoff notes)
            </div>
            <ul v-else class="space-y-1 mt-1 text-[12px]">
              <li v-for="h in digest.handoffs" :key="h.handoffId">
                [{{ h.status }}] {{ h.handoffId }} → {{ h.recipientLabel }}
              </li>
            </ul>
          </div>
        </div>
      </div>

      <!-- Mesh -->
      <div v-else-if="tab === 'mesh'" class="space-y-3">
        <div class="workbench-card p-4 space-y-3">
          <h3 class="section-label">Peers ({{ peers.length }})</h3>
          <div v-if="peers.length === 0" class="text-[12px] text-muted">
            No mesh peers registered. Use CLI
            <code class="code">mesh peer add</code> to register a local peer.
          </div>
          <div
            v-for="p in peers"
            :key="p.peerId"
            class="rounded-lg p-3 text-[12px] cursor-pointer"
            style="background: var(--surface-2); border: 1px solid var(--border)"
            @click="meshPeer = p.peerId"
          >
            <div class="font-medium text-[13px]">{{ p.label }}</div>
            <div class="text-muted">id={{ p.peerId }}</div>
            <div v-if="p.remoteWorkspaceId" class="text-muted">
              workspace={{ p.remoteWorkspaceId }}
            </div>
          </div>
        </div>
        <div class="workbench-card p-4 space-y-3">
          <h3 class="section-label">Ask offline peer (Ter × Yo)</h3>
          <p class="text-[12px] text-muted">
            Read-only remote query against imported mesh envelopes. Does not
            write the local ledger.
          </p>
          <input
            v-model="meshPeer"
            class="input-sm w-full"
            placeholder="Peer id or label (e.g. yo)"
          />
          <textarea
            v-model="meshQuestion"
            rows="2"
            class="input-area w-full"
            placeholder="What did Yo finish while offline?"
          />
          <div class="flex items-center gap-2">
            <select v-model="meshTier" class="input-sm">
              <option value="low-impact">low-impact</option>
              <option value="standard">standard</option>
              <option value="critical">critical</option>
            </select>
            <button
              type="button"
              class="btn-primary"
              :disabled="acting || !meshPeer.trim() || !meshQuestion.trim()"
              @click="handleMeshQuery"
            >
              Query peer
            </button>
          </div>
          <div
            v-if="meshAnswer"
            class="rounded-lg p-3 space-y-2"
            style="background: var(--surface-2); border: 1px solid var(--border)"
          >
            <div class="flex items-center gap-2 text-[12px]">
              <span class="badge">{{ meshAnswer.peerLabel }}</span>
              <span v-if="meshAnswer.readOnly" class="badge">read-only</span>
              <span v-if="meshAnswer.refused" class="badge">refused</span>
            </div>
            <div class="text-[12px] text-muted">{{ meshAnswer.freshness.statement }}</div>
            <pre class="text-[12px] whitespace-pre-wrap font-sans">{{
              meshAnswer.answerText
            }}</pre>
          </div>
        </div>
        <div class="workbench-card p-4 space-y-3">
          <h3 class="section-label">Envelopes ({{ envelopes.length }})</h3>
          <div v-if="envelopes.length === 0" class="text-[12px] text-muted">
            No inbox/outbox envelopes. Use CLI
            <code class="code">mesh export</code> /
            <code class="code">mesh import</code>.
          </div>
          <div
            v-for="e in envelopes"
            :key="e.mailbox + e.envelopeId"
            class="rounded-lg p-3 text-[12px]"
            style="background: var(--surface-2); border: 1px solid var(--border)"
          >
            <div class="flex items-center gap-2">
              <span class="badge">{{ e.mailbox }}</span>
              <span class="font-medium">{{ e.envelopeId }}</span>
            </div>
            <div class="text-muted mt-1">
              from {{ e.attributedTo }} · evidence {{ e.evidenceItemCount }} ·
              {{ e.generatedAt }}
            </div>
          </div>
        </div>
      </div>

      <!-- LAN -->
      <div v-else-if="tab === 'lan'" class="space-y-3">
        <div class="workbench-card p-4 space-y-3">
          <h3 class="section-label">LAN listener</h3>
          <p class="text-[12px] text-muted">
            Trusted-LAN alpha: UDP discovery + HTTP package transfer / live ask.
            macOS may prompt for firewall on first bind. UDP can fail on some
            VPN interfaces — use manual host:port.
          </p>
          <div v-if="lanStatus?.running" class="flex items-center gap-2 text-[13px]">
            <CheckCircle2 class="h-4 w-4" style="color: var(--accent-green)" />
            <span class="font-medium">Listening</span>
            <span class="badge">{{ lanStatus.httpHost }}:{{ lanStatus.httpPort }}</span>
          </div>
          <div v-else class="text-[12px] text-muted">Listener is stopped.</div>
          <div class="flex items-center gap-2">
            <button
              type="button"
              class="btn-primary"
              :disabled="acting || !!lanStatus?.running"
              @click="handleLanStart"
            >
              Start listener
            </button>
            <button
              type="button"
              class="btn-ghost"
              :disabled="acting || !lanStatus?.running"
              @click="handleLanStop"
            >
              Stop
            </button>
            <button
              type="button"
              class="btn-ghost"
              :disabled="acting"
              @click="handleLanDiscover"
            >
              Refresh discover
            </button>
          </div>
          <div v-if="lanStatus?.note" class="text-[11px] text-muted">{{ lanStatus.note }}</div>
        </div>
        <div class="workbench-card p-4 space-y-3">
          <h3 class="section-label">Peers ({{ lanPeers.length }})</h3>
          <div v-if="lanPeers.length === 0" class="text-[12px] text-muted">
            No peers discovered yet. Start a listener on another device, then
            Refresh discover — or enter host:port manually below.
          </div>
          <div
            v-for="p in lanPeers"
            :key="p.peerId + p.address"
            class="rounded-lg p-3 text-[12px] cursor-pointer"
            style="background: var(--surface-2); border: 1px solid var(--border)"
            @click="selectLanPeer(p)"
          >
            <div class="font-medium text-[13px]">{{ p.ownerLabel }}</div>
            <div class="text-muted">id={{ p.peerId }} · {{ p.address }}</div>
            <div class="text-muted text-[11px]">seen {{ p.lastSeenAt }}</div>
          </div>
        </div>
        <div class="workbench-card p-4 space-y-3">
          <h3 class="section-label">Send approved package</h3>
          <input
            v-model="lanSendId"
            class="input-sm w-full"
            placeholder="Package id (approved)"
          />
          <input
            v-model="lanSendTo"
            class="input-sm w-full"
            placeholder="Peer host:port"
          />
          <button
            type="button"
            class="btn-primary"
            :disabled="acting || !lanSendId.trim() || !lanSendTo.trim()"
            @click="handleLanSend"
          >
            Send over LAN
          </button>
        </div>
        <div class="workbench-card p-4 space-y-3">
          <h3 class="section-label">Ask peer (live)</h3>
          <p class="text-[12px] text-muted">
            Read-only draft from the peer’s local Work Proxy while they are
            online. Does not write the local ledger.
          </p>
          <input
            v-model="lanAskTo"
            class="input-sm w-full"
            placeholder="Peer host:port"
          />
          <textarea
            v-model="lanAskQuestion"
            rows="2"
            class="input-area w-full"
            placeholder="What is in progress?"
          />
          <div class="flex items-center gap-2">
            <select v-model="lanAskTier" class="input-sm">
              <option value="low-impact">low-impact</option>
              <option value="standard">standard</option>
              <option value="critical">critical</option>
            </select>
            <button
              type="button"
              class="btn-primary"
              :disabled="acting || !lanAskTo.trim() || !lanAskQuestion.trim()"
              @click="handleLanAsk"
            >
              Ask peer
            </button>
          </div>
          <div
            v-if="lanAskAnswer"
            class="rounded-lg p-3 space-y-2"
            style="background: var(--surface-2); border: 1px solid var(--border)"
          >
            <div class="flex items-center gap-2 text-[12px]">
              <span class="badge">{{ lanAskAnswer.peerLabel }}</span>
              <span v-if="lanAskAnswer.readOnly" class="badge">read-only</span>
              <span v-if="lanAskAnswer.refused" class="badge">refused</span>
            </div>
            <div class="text-[12px] text-muted">{{ lanAskAnswer.freshness.statement }}</div>
            <pre class="text-[12px] whitespace-pre-wrap font-sans">{{
              lanAskAnswer.answerText
            }}</pre>
          </div>
        </div>
      </div>

      <!-- Relay -->
      <div v-else-if="tab === 'relay'" class="space-y-3">
        <div class="workbench-card p-4 space-y-3">
          <h3 class="section-label">Audit trail ({{ audit.length }})</h3>
          <p class="text-[12px] text-muted">
            Pack / approve / send / receive remain CLI-first. This view is
            read-only audit.
          </p>
          <div v-if="audit.length === 0" class="text-[12px] text-muted py-2">
            No relay audit events yet.
          </div>
          <div
            v-for="ev in audit"
            :key="ev.eventId"
            class="rounded-lg p-3 text-[12px]"
            style="background: var(--surface-2); border: 1px solid var(--border)"
          >
            <div class="flex items-center gap-2">
              <span class="badge">{{ ev.kind }}</span>
              <span class="font-medium">{{ ev.packageId }}</span>
            </div>
            <div class="text-muted mt-1">{{ ev.detail }}</div>
            <div class="text-muted text-[11px] mt-0.5">
              {{ ev.at }}
              <span v-if="ev.actorLabel"> · {{ ev.actorLabel }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Online proxy -->
      <div v-else-if="tab === 'online-proxy'" class="space-y-3">
        <div class="workbench-card p-4 space-y-3">
          <template v-if="!onlineConfig">
            <p class="text-[13px] text-muted">
              Always-online proxy is not initialized for this project.
            </p>
            <button
              type="button"
              class="btn-primary"
              :disabled="acting"
              @click="handleInitProxy"
            >
              Initialize local scaffold
            </button>
          </template>
          <template v-else>
            <div class="flex items-center gap-2 text-[13px]">
              <CheckCircle2 class="h-4 w-4" style="color: var(--accent-green)" />
              <span class="font-medium">{{ onlineConfig.proxyId }}</span>
              <span class="badge">{{ onlineConfig.mode }}</span>
            </div>
            <div class="text-[12px] text-muted">
              owner={{ onlineConfig.ownerLabel }} · relay-received={{
                onlineConfig.useRelayReceived
              }}
              · default tier={{ onlineConfig.defaultFreshnessTier }}
            </div>
            <div class="space-y-2 pt-2">
              <label class="text-[12px] font-medium">Ask (mandatory freshness)</label>
              <textarea
                v-model="askQuestion"
                rows="3"
                class="input-area w-full"
                placeholder="What needs attention while I was away?"
              />
              <div class="flex items-center gap-2">
                <select v-model="askTier" class="input-sm">
                  <option value="low-impact">low-impact</option>
                  <option value="standard">standard</option>
                  <option value="critical">critical</option>
                </select>
                <button
                  type="button"
                  class="btn-primary"
                  :disabled="acting || !askQuestion.trim()"
                  @click="handleAsk"
                >
                  Ask
                </button>
              </div>
            </div>
            <div
              v-if="lastAnswer"
              class="rounded-lg p-3 space-y-2 mt-2"
              style="background: var(--surface-2); border: 1px solid var(--border)"
            >
              <div class="flex items-center gap-2 text-[12px]">
                <ShieldAlert
                  v-if="lastAnswer.refused"
                  class="h-4 w-4"
                  style="color: var(--accent-amber)"
                />
                <CheckCircle2
                  v-else
                  class="h-4 w-4"
                  style="color: var(--accent-green)"
                />
                <span class="font-medium">{{ lastAnswer.answerId }}</span>
                <span v-if="lastAnswer.refused" class="badge">refused</span>
              </div>
              <div
                class="text-[12px] rounded p-2"
                style="background: var(--surface-3); color: var(--muted-foreground)"
              >
                {{ lastAnswer.freshness.statement }}
              </div>
              <pre class="text-[12px] whitespace-pre-wrap font-sans">{{
                lastAnswer.answerText
              }}</pre>
            </div>
          </template>
        </div>
      </div>

      <!-- Team (0.1.15) -->
      <div v-else-if="tab === 'team'" class="space-y-4">
        <div v-if="loading" class="text-[13px] text-muted flex items-center gap-2">
          <Loader2 class="h-4 w-4 animate-spin" /> Loading…
        </div>
        <div v-else-if="!teamWs" class="workbench-card p-8 text-center space-y-2">
          <p class="text-[14px] font-semibold">No team workspace</p>
          <p class="text-[12px] text-muted">Run <code class="text-[11px]">team init</code> in the CLI for this project.</p>
        </div>
        <div v-else class="workbench-card p-5 space-y-3">
          <h3 class="text-[14px] font-semibold">{{ teamWs.displayName }}</h3>
          <p class="text-[12px] text-muted">team_id={{ teamWs.teamId }} · members={{ teamWs.members?.length ?? 0 }}</p>
          <ul class="space-y-1.5 text-[12px]">
            <li v-for="m in teamWs.members" :key="m.memberId" class="flex gap-2">
              <span class="font-medium">{{ m.label }}</span>
              <span class="text-muted">{{ m.role }}</span>
              <span v-if="m.meshPeerId" class="text-muted">peer={{ m.meshPeerId }}</span>
            </li>
          </ul>
        </div>
      </div>

      <!-- Trust (0.1.17) -->
      <div v-else-if="tab === 'trust'" class="space-y-4">
        <div v-if="loading" class="text-[13px] text-muted flex items-center gap-2">
          <Loader2 class="h-4 w-4 animate-spin" /> Loading…
        </div>
        <div v-else-if="!trustPolicy" class="workbench-card p-8 text-center space-y-2">
          <p class="text-[14px] font-semibold">No trust policy</p>
          <p class="text-[12px] text-muted">Run <code class="text-[11px]">trust-admin init</code> after team init.</p>
        </div>
        <div v-else class="workbench-card p-5 space-y-2 text-[12px]">
          <p><span class="text-muted">remote query</span> · {{ trustPolicy.remoteQueryEnabled ? 'enabled' : 'disabled' }}</p>
          <p><span class="text-muted">allowlist mode</span> · {{ trustPolicy.queryAllowlistMode }}</p>
          <p><span class="text-muted">allowlist size</span> · {{ trustPolicy.queryAllowlist?.length ?? 0 }}</p>
          <p><span class="text-muted">secrets fail-closed</span> · {{ trustPolicy.secretTopicsFailClosed }}</p>
          <p><span class="text-muted">secret export</span> · {{ trustPolicy.allowSecretExport }}</p>
          <p><span class="text-muted">selective sync</span> · {{ trustPolicy.syncRequireSelective }}</p>
        </div>
      </div>

      <!-- Connectors (0.1.18) -->
      <div v-else-if="tab === 'connectors'" class="space-y-4">
        <div v-if="loading" class="text-[13px] text-muted flex items-center gap-2">
          <Loader2 class="h-4 w-4 animate-spin" /> Loading…
        </div>
        <div v-else-if="!connectors.length" class="workbench-card p-8 text-center space-y-2">
          <p class="text-[14px] font-semibold">No connectors registered</p>
          <p class="text-[12px] text-muted">Run <code class="text-[11px]">connector register --id gh-lab --kind github-stub</code>.</p>
        </div>
        <div v-else class="space-y-2">
          <div v-for="c in connectors" :key="c.connectorId" class="workbench-card-compact p-4 text-[12px]">
            <div class="flex items-center justify-between">
              <span class="font-semibold">{{ c.displayName }}</span>
              <span class="chip">{{ c.kind }}</span>
            </div>
            <p class="text-muted mt-1">{{ c.connectorId }} · {{ c.enabled ? 'enabled' : 'disabled' }} · {{ c.role }}</p>
            <p v-if="c.externalRef" class="text-muted">ref={{ c.externalRef }}</p>
          </div>
        </div>
      </div>



      <!-- RC (0.1.21) -->
      <div v-else-if="tab === 'rc'" class="space-y-4">
        <div v-if="loading" class="text-[13px] text-muted flex items-center gap-2">
          <Loader2 class="h-4 w-4 animate-spin" /> Loading…
        </div>
        <div v-else-if="!rcPack" class="workbench-card p-8 text-center text-[13px] text-muted">
          Could not evaluate RC pack.
        </div>
        <template v-else>
          <div class="workbench-card p-4 flex flex-wrap gap-2 text-[12px]">
            <span class="chip">{{ rcPack.rcReady ? 'rc ready' : 'not rc ready' }}</span>
            <span class="chip">p0 fail {{ rcPack.p0FailCount }}</span>
            <span class="chip">p1 fail {{ rcPack.p1FailCount }}</span>
          </div>
          <div class="workbench-card p-4 space-y-2 text-[12px]">
            <h4 class="font-semibold">Checks</h4>
            <div v-for="c in rcPack.checks" :key="c.id" class="border-b pb-2" style="border-color: var(--border)">
              <span class="chip mr-1">{{ c.severity }}</span>
              <span class="chip mr-1">{{ c.status }}</span>
              <span class="font-medium">{{ c.title }}</span>
              <p class="text-muted">{{ c.evidence }}</p>
            </div>
          </div>
          <div class="workbench-card p-4 space-y-2 text-[12px]">
            <h4 class="font-semibold">Regression matrix</h4>
            <div v-for="r in rcPack.regressionMatrix" :key="r.id">
              <span class="chip mr-1">{{ r.status }}</span>
              {{ r.area }} · {{ r.surface }}
              <span class="text-muted"> — {{ r.evidence }}</span>
            </div>
          </div>
          <div class="workbench-card p-4 text-[12px] space-y-1">
            <h4 class="font-semibold">Freeze policy</h4>
            <p>{{ rcPack.freezePolicy.summary }}</p>
            <p class="text-muted">features_frozen={{ rcPack.freezePolicy.featuresFrozen }}</p>
          </div>
        </template>
      </div>

      <!-- Pilot (0.1.20) -->
      <div v-else-if="tab === 'pilot'" class="space-y-4">
        <div v-if="loading" class="text-[13px] text-muted flex items-center gap-2">
          <Loader2 class="h-4 w-4 animate-spin" /> Loading…
        </div>
        <div v-else-if="!pilotPack" class="workbench-card p-8 text-center text-[13px] text-muted">
          Could not evaluate pilot pack.
        </div>
        <template v-else>
          <div class="workbench-card p-4 flex flex-wrap gap-2 items-center text-[12px]">
            <span class="chip" :class="pilotPack.pilotReady ? 'chip-info' : ''">
              {{ pilotPack.pilotReady ? 'pilot ready' : 'not ready' }}
            </span>
            <span class="chip">pass {{ pilotPack.passCount }}</span>
            <span class="chip">warn {{ pilotPack.warnCount }}</span>
            <span class="chip">fail {{ pilotPack.failCount }}</span>
          </div>
          <div class="workbench-card p-4 space-y-2">
            <h4 class="text-[12px] font-semibold">Checks</h4>
            <div v-for="c in pilotPack.checks" :key="c.id" class="text-[12px] border-b pb-2" style="border-color: var(--border)">
              <div class="flex gap-2 items-center">
                <span class="chip">{{ c.status }}</span>
                <span class="font-medium">{{ c.title }}</span>
              </div>
              <p class="text-muted mt-0.5">{{ c.evidence }}</p>
              <p v-if="c.detail" class="text-muted opacity-80">{{ c.detail }}</p>
            </div>
          </div>
          <div class="workbench-card p-4 space-y-2">
            <h4 class="text-[12px] font-semibold">Threat notes</h4>
            <div v-for="t in pilotPack.threatNotes" :key="t.id" class="text-[12px]">
              <p class="font-medium">{{ t.title }}</p>
              <p class="text-muted">{{ t.summary }}</p>
              <p class="text-muted opacity-80">Residual: {{ t.residual }}</p>
            </div>
          </div>
          <div class="workbench-card p-4 space-y-2">
            <h4 class="text-[12px] font-semibold">Runbook</h4>
            <div v-for="s in pilotPack.runbook" :key="s.id" class="text-[12px]">
              <p class="font-medium">{{ s.title }}</p>
              <p class="font-mono text-[11px] text-muted">{{ s.commandOrAction }}</p>
              <p class="text-muted">{{ s.purpose }}</p>
            </div>
          </div>
        </template>
      </div>

      <!-- Org graph (0.1.19) -->
      <div v-else-if="tab === 'org'" class="space-y-4">
        <div v-if="loading" class="text-[13px] text-muted flex items-center gap-2">
          <Loader2 class="h-4 w-4 animate-spin" /> Loading…
        </div>
        <div v-else-if="!orgGraph" class="workbench-card p-8 text-center space-y-2">
          <p class="text-[14px] font-semibold">No org graph yet</p>
          <p class="text-[12px] text-muted">Initialize a team workspace, then refresh. Graph is evidence-backed only.</p>
        </div>
        <div v-else class="space-y-4">
          <div class="workbench-card p-4 text-[12px]">
            <p class="font-semibold">team {{ orgGraph.teamId }}</p>
            <p class="text-muted">generated {{ orgGraph.generatedAt }} · nodes {{ orgGraph.nodes?.length ?? 0 }} · edges {{ orgGraph.edges?.length ?? 0 }}</p>
          </div>
          <div class="workbench-card p-4">
            <h4 class="text-[12px] font-semibold mb-2">Nodes</h4>
            <ul class="space-y-1 text-[12px]">
              <li v-for="n in orgGraph.nodes" :key="n.id">
                <span class="chip mr-1">{{ n.kind }}</span>
                <span class="font-medium">{{ n.label }}</span>
                <span class="text-muted"> · {{ n.evidence }}</span>
              </li>
            </ul>
          </div>
          <div class="workbench-card p-4">
            <h4 class="text-[12px] font-semibold mb-2">Edges</h4>
            <ul class="space-y-1 text-[12px] text-muted">
              <li v-for="(e, i) in orgGraph.edges" :key="i">
                {{ e.from }} —{{ e.kind }}→ {{ e.to }}
                <span class="opacity-70">({{ e.evidence }})</span>
              </li>
            </ul>
          </div>
        </div>
      </div>

    </template>
  </div>
</template>

<style scoped>
.cont {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  max-width: 920px;
}

.cont__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.cont__title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 650;
  letter-spacing: -0.02em;
  line-height: 1.2;
}

.cont__meta {
  margin: 0.2rem 0 0;
  font-size: 0.78rem;
  color: var(--muted-foreground);
  font-variant-numeric: tabular-nums;
}

.cont__refresh {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--foreground);
  cursor: pointer;
  flex-shrink: 0;
}

.cont__refresh:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.cont__empty {
  text-align: center;
  padding: 2.25rem 1rem;
}

.cont__empty-title {
  margin: 0;
  font-size: 0.95rem;
  font-weight: 600;
}

.cont__empty-body {
  margin: 0.35rem 0 0;
  font-size: 0.8rem;
  color: var(--muted-foreground);
}

.chip {
  display: inline-flex;
  align-items: center;
  padding: 0.25rem 0.6rem;
  border-radius: 999px;
  font-size: 11px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  color: var(--muted-foreground);
}
.badge {
  display: inline-flex;
  padding: 0.1rem 0.4rem;
  border-radius: 4px;
  font-size: 10px;
  text-transform: lowercase;
  background: var(--surface-3);
  color: var(--muted-foreground);
}
.section-label {
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: var(--muted-foreground);
}
.code {
  font-family: var(--font-mono);
  font-size: 11px;
  padding: 0.05rem 0.3rem;
  border-radius: 3px;
  background: var(--surface-3);
}
.input-sm {
  font-size: 12px;
  padding: 0.35rem 0.5rem;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--foreground);
}
.input-area {
  font-size: 13px;
  padding: 0.6rem 0.75rem;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--foreground);
  resize: vertical;
}
.btn-primary {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  font-size: 12px;
  font-weight: 600;
  padding: 0.45rem 0.9rem;
  border-radius: 8px;
  background: var(--accent-blue);
  color: #fff;
  border: none;
  cursor: pointer;
}
.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
