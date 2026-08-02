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
  type MeshRemoteQueryAnswer,
} from "../lib/continuityClient";

type TabId = "pending" | "digest" | "mesh" | "relay" | "online-proxy" | "team" | "trust" | "connectors" | "org";

const { currentProject, currentProjectPath } = useStore();

const tab = ref<TabId>("pending");
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
const acting = ref(false);
const teamWs = ref<TeamWorkspaceView | null>(null);
const trustPolicy = ref<TeamTrustPolicyView | null>(null);
const connectors = ref<ConnectorDescriptorView[]>([]);
const orgGraph = ref<OrgGraphView | null>(null);

const hasProject = computed(() => !!currentProjectPath.value);

const tabs: { id: TabId; label: string; icon: typeof Inbox }[] = [
  { id: "pending", label: "Pending", icon: Inbox },
  { id: "digest", label: "Digest", icon: Clock },
  { id: "mesh", label: "Mesh", icon: Users },
  { id: "team", label: "Team", icon: Users },
  { id: "trust", label: "Trust", icon: Shield },
  { id: "connectors", label: "Connectors", icon: Plug },
  { id: "org", label: "Org", icon: Network },
  { id: "relay", label: "Relay", icon: Share2 },
  { id: "online-proxy", label: "Online Proxy", icon: Cloud },
];

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
  loadTab();
});

onMounted(() => {
  loadTab();
});
</script>

<template>
  <div class="space-y-6 animate-fade-in">
    <div class="flex items-start justify-between gap-4">
      <div>
        <h1 class="text-title">Continuity</h1>
        <p class="text-body text-muted mt-1">
          Mesh, team, trust, connectors, and org graph — desktop surfaces for continuity through 0.1.19.
        </p>
      </div>
      <button
        type="button"
        class="inline-flex items-center gap-2 rounded-lg px-3 py-2 text-[12px] font-medium"
        style="
          background: var(--surface-2);
          border: 1px solid var(--border);
          color: var(--foreground);
        "
        :disabled="!hasProject || loading"
        @click="refresh"
      >
        <RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': loading }" />
        Refresh
      </button>
    </div>

    <div
      v-if="!hasProject"
      class="workbench-card p-8 text-center text-[13px] text-muted"
    >
      No project selected. Choose a project in the sidebar to view continuity.
    </div>

    <template v-else>
      <!-- Summary chips -->
      <div v-if="summary" class="flex flex-wrap gap-2">
        <span class="chip">Pending open: {{ summary.openPendingCount }}</span>
        <span class="chip">Peers: {{ summary.peerCount }}</span>
        <span class="chip">Envelopes: {{ summary.envelopeCount }}</span>
        <span class="chip">Relay audit: {{ summary.auditEventCount }}</span>
        <span class="chip">
          Online proxy:
          {{ summary.onlineProxyInitialized ? "ready" : "not init" }}
        </span>
      </div>

      <!-- Tabs -->
      <div class="flex flex-wrap gap-1 border-b pb-0" style="border-color: var(--border)">
        <button
          v-for="t in tabs"
          :key="t.id"
          type="button"
          class="tab-btn inline-flex items-center gap-1.5 px-3 py-2 text-[12px] font-medium"
          :class="{ active: tab === t.id }"
          @click="tab = t.id"
        >
          <component :is="t.icon" class="h-3.5 w-3.5" />
          {{ t.label }}
        </button>
      </div>

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
        <div v-if="pending" class="workbench-card p-4 space-y-3">
          <div class="flex items-center justify-between text-[12px] text-muted">
            <span>Open: {{ pending.openCount }}</span>
            <span>
              proxy {{ pending.sourceCounts.proxyPending }} · attention
              {{ pending.sourceCounts.continuityAttention }} · signal
              {{ pending.sourceCounts.unresolvedSignal }}
            </span>
          </div>
          <div v-if="pending.items.length === 0" class="text-[13px] text-muted py-4 text-center">
            Nothing pending — you are clear.
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
.tab-btn {
  color: var(--muted-foreground);
  border-bottom: 2px solid transparent;
  margin-bottom: -1px;
}
.tab-btn.active {
  color: var(--foreground);
  border-bottom-color: var(--accent-blue);
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
