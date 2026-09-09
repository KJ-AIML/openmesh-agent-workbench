<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
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
  MessageSquare,
} from "lucide-vue-next";
import { useStore } from "../lib/useStore";
import ChatMessageContent from "../components/chat/ChatMessageContent.vue";
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
  type LanPeerPresence,
  type LanPresenceState,
  type LanChatMessageView,
  getContinuityHubSummary,
  getPendingQuestions,
  getReturnDigest,
  listMeshPeers,
  listMeshEnvelopes,
  queryMeshPeer,
  addMeshPeer,
  listRelayAudit,
  getOnlineProxyStatus,
  initOnlineProxy,
  askOnlineProxy,
  getTeamWorkspace,
  getTeamTrustPolicy,
  initTeamWorkspace,
  addTeamMember,
  removeTeamMember,
  initTeamTrustPolicy,
  setTeamTrustRemoteQuery,
  setTeamTrustQueryMode,
  addTeamTrustAllowlist,
  removeTeamTrustAllowlist,
  listTeamTrustAudit,
  listConnectors,
  getOrgGraph,
  getPilotStatus,
  getRcStatus,
  lanServeStart,
  lanServeStop,
  lanServeStatus,
  lanDiscover,
  lanListApprovedPackages,
  lanSendPackage,
  lanAskPeer,
  lanProbePresence,
  lanProbeAddress,
  lanChatSend,
  lanChatList,
  type MeshRemoteQueryAnswer,
} from "../lib/continuityClient";

type TabId =
  | "pending"
  | "digest"
  | "mesh"
  | "relay"
  | "online-proxy"
  | "lan"
  | "chat"
  | "team"
  | "trust"
  | "connectors"
  | "org"
  | "pilot"
  | "rc";
type GroupId = "attention" | "share" | "advanced";

const { currentProject, currentProjectPath } = useStore();

const tab = ref<TabId>("pending");
const group = ref<GroupId>("attention");
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
const lanApprovedPackages = ref<string[]>([]);
const lanSendId = ref("");
const lanSendTo = ref("");
const lanAskTo = ref("");
const lanAskQuestion = ref("");
const lanAskTier = ref("low-impact");
const lanAskAnswer = ref<MeshRemoteQueryAnswer | null>(null);
const lanPresenceByAddress = ref<Record<string, LanPeerPresence>>({});
const lanManualProbe = ref("");
const lanManualProbeResult = ref<LanPeerPresence | null>(null);
const presenceProbing = ref(false);
let presenceTimer: ReturnType<typeof setInterval> | null = null;
const packCliHint = computed(
  () =>
    "openmesh-cli relay pack --envelope-id <id> --package-id <pkg> && openmesh-cli relay approve --id <pkg>",
);
const acting = ref(false);
const teamWs = ref<TeamWorkspaceView | null>(null);
const trustPolicy = ref<TeamTrustPolicyView | null>(null);
const trustAudit = ref<
  Array<{
    eventId: string;
    teamId: string;
    actorMemberId: string;
    action: string;
    detail: string;
    at: string;
  }>
>([]);
const connectors = ref<ConnectorDescriptorView[]>([]);
const orgGraph = ref<OrgGraphView | null>(null);
const pilotPack = ref<PilotPackView | null>(null);
const rcPack = ref<RcPackView | null>(null);

const meshAddLabel = ref("");
const meshAddLan = ref("");
const meshAddNotes = ref("");
const teamInitName = ref("");
const teamMemberLabel = ref("");
const teamMemberPeer = ref("");
const trustAllowPeer = ref("");
const chatPeerTo = ref("");
const chatText = ref("");
const chatMessages = ref<LanChatMessageView[]>([]);

const hasProject = computed(() => !!currentProjectPath.value);

const groups: { id: GroupId; label: string; tabs: TabId[] }[] = [
  { id: "attention", label: "Attention", tabs: ["pending", "digest"] },
  { id: "share", label: "Share", tabs: ["lan", "relay", "chat"] },
  {
    id: "advanced",
    label: "Advanced",
    tabs: [
      "mesh",
      "team",
      "trust",
      "connectors",
      "org",
      "online-proxy",
      "pilot",
      "rc",
    ],
  },
];

const tabMeta: Record<TabId, { label: string; icon: typeof Inbox }> = {
  pending: { label: "Pending", icon: Inbox },
  digest: { label: "Digest", icon: Clock },
  mesh: { label: "Peers", icon: Users },
  team: { label: "Team", icon: Users },
  trust: { label: "Trust", icon: Shield },
  connectors: { label: "Connectors", icon: Plug },
  org: { label: "Org", icon: Network },
  pilot: { label: "Pilot", icon: ClipboardCheck },
  rc: { label: "RC", icon: Flag },
  relay: { label: "Relay", icon: Share2 },
  "online-proxy": { label: "Proxy", icon: Cloud },
  lan: { label: "LAN", icon: Radio },
  chat: { label: "Chat", icon: MessageSquare },
};

/** LAN dogfood path — short steps, not a card dashboard. */
const lanDogfoodSteps: Array<{ id: TabId; label: string; hint: string }> = [
  { id: "mesh", label: "Peers", hint: "Register a peer (+ optional LAN host:port)" },
  { id: "team", label: "Team", hint: "Init workspace and link members to peers" },
  { id: "trust", label: "Trust", hint: "Allowlist the peer for remote query" },
  { id: "lan", label: "LAN", hint: "Start listener / discover on the same network" },
];

const showLanDogfoodGuide = computed(() =>
  lanDogfoodSteps.some((s) => s.id === tab.value),
);

const lanDogfoodDone = computed<Record<string, boolean>>(() => ({
  mesh: peers.value.length > 0,
  team: !!teamWs.value,
  trust: !!trustPolicy.value,
  lan: !!lanStatus.value?.running,
}));

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
        teamWs.value = await getTeamWorkspace(path);
        trustPolicy.value = await getTeamTrustPolicy(path);
        lanStatus.value = await lanServeStatus(path);
        if (peers.value.some((p) => p.lanAddress)) {
          await refreshLanPresence();
        }
        break;
      case "relay":
        audit.value = await listRelayAudit(path);
        break;
      case "online-proxy":
        onlineConfig.value = await getOnlineProxyStatus(path);
        break;
      case "lan":
        lanStatus.value = await lanServeStatus(path);
        peers.value = await listMeshPeers(path);
        teamWs.value = await getTeamWorkspace(path);
        trustPolicy.value = await getTeamTrustPolicy(path);
        await refreshLanApproved();
        if (lanPeers.value.length === 0) {
          try {
            lanPeers.value = await lanDiscover(path, { seconds: 2 });
          } catch {
            /* keep empty */
          }
        }
        await refreshLanPresence();
        startPresencePolling();
        break;
      case "chat":
        lanStatus.value = await lanServeStatus(path);
        if (lanPeers.value.length === 0) {
          try {
            lanPeers.value = await lanDiscover(path, { seconds: 2 });
          } catch {
            /* keep empty */
          }
        }
        await refreshLanPresence();
        await refreshChatMessages();
        startPresencePolling();
        break;
      case "team":
        teamWs.value = await getTeamWorkspace(path);
        peers.value = await listMeshPeers(path);
        trustPolicy.value = await getTeamTrustPolicy(path);
        lanStatus.value = await lanServeStatus(path);
        if (peers.value.some((p) => p.lanAddress)) {
          await refreshLanPresence();
        }
        break;
      case "trust":
        trustPolicy.value = await getTeamTrustPolicy(path);
        teamWs.value = await getTeamWorkspace(path);
        peers.value = await listMeshPeers(path);
        lanStatus.value = await lanServeStatus(path);
        try {
          trustAudit.value = await listTeamTrustAudit(path, 20);
        } catch {
          trustAudit.value = [];
        }
        if (peers.value.some((p) => p.lanAddress)) {
          await refreshLanPresence();
        }
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

async function copyText(text: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    error.value = "Could not copy to clipboard";
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
    // Falls back to last-known peers when UDP finds nothing (VPN/loopback).
    lanPeers.value = await lanDiscover(currentProjectPath.value, { seconds: 3 });
    await refreshLanPresence();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

function presenceFor(address: string): LanPeerPresence | undefined {
  return lanPresenceByAddress.value[address];
}

function presenceLabel(state?: LanPresenceState): string {
  switch (state) {
    case "live":
      return "live";
    case "stale":
      return "stale";
    case "unreachable":
      return "unreachable";
    default:
      return "unknown";
  }
}

function peerById(peerId?: string | null): MeshPeerRecord | undefined {
  if (!peerId) return undefined;
  return peers.value.find((p) => p.peerId === peerId);
}

function presenceForPeerId(peerId?: string | null): LanPeerPresence | undefined {
  const p = peerById(peerId);
  return p?.lanAddress ? presenceFor(p.lanAddress) : undefined;
}

function canRemoveMember(memberId: string, role: string): boolean {
  if (!teamWs.value?.members?.length) return false;
  if (role === "owner") {
    const owners = teamWs.value.members.filter((m) => m.role === "owner");
    return owners.length > 1;
  }
  return true;
}

async function refreshLanPresence() {
  const targets: Array<{ address: string; lastSeenAt?: string }> = [];
  const seen = new Set<string>();
  for (const p of lanPeers.value) {
    if (!p.address || seen.has(p.address)) continue;
    seen.add(p.address);
    targets.push({ address: p.address, lastSeenAt: p.lastSeenAt });
  }
  for (const p of peers.value) {
    if (p.lanAddress && !seen.has(p.lanAddress)) {
      seen.add(p.lanAddress);
      targets.push({ address: p.lanAddress });
    }
  }
  if (lanAskTo.value.trim() && !seen.has(lanAskTo.value.trim())) {
    targets.push({ address: lanAskTo.value.trim() });
  }
  if (lanSendTo.value.trim() && !seen.has(lanSendTo.value.trim())) {
    targets.push({ address: lanSendTo.value.trim() });
  }
  if (targets.length === 0) return;
  presenceProbing.value = true;
  try {
    const rows = await lanProbePresence(targets);
    const next: Record<string, LanPeerPresence> = {
      ...lanPresenceByAddress.value,
    };
    for (const row of rows) {
      next[row.address] = row;
    }
    lanPresenceByAddress.value = next;
  } catch {
    /* keep prior presence */
  } finally {
    presenceProbing.value = false;
  }
}

function startPresencePolling() {
  stopPresencePolling();
  presenceTimer = setInterval(() => {
    if (tab.value === "lan" || tab.value === "chat") {
      void refreshLanPresence();
    }
  }, 8000);
}

function stopPresencePolling() {
  if (presenceTimer) {
    clearInterval(presenceTimer);
    presenceTimer = null;
  }
}

async function handleManualProbe() {
  if (!lanManualProbe.value.trim()) return;
  acting.value = true;
  error.value = null;
  lanManualProbeResult.value = null;
  try {
    const row = await lanProbeAddress(lanManualProbe.value.trim());
    lanManualProbeResult.value = row;
    lanPresenceByAddress.value = {
      ...lanPresenceByAddress.value,
      [row.address]: row,
    };
    lanAskTo.value = row.address;
    lanSendTo.value = row.address;
    chatPeerTo.value = row.address;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function handleMeshAddPeer() {
  if (!currentProjectPath.value || !meshAddLabel.value.trim()) return;
  acting.value = true;
  error.value = null;
  try {
    await addMeshPeer(currentProjectPath.value, {
      label: meshAddLabel.value.trim(),
      lanAddress: meshAddLan.value.trim() || undefined,
      notes: meshAddNotes.value.trim() || undefined,
    });
    meshAddLabel.value = "";
    meshAddLan.value = "";
    meshAddNotes.value = "";
    peers.value = await listMeshPeers(currentProjectPath.value);
    await loadSummary();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function handleTeamInit() {
  if (!currentProjectPath.value || !teamInitName.value.trim()) return;
  acting.value = true;
  error.value = null;
  try {
    teamWs.value = await initTeamWorkspace(currentProjectPath.value, {
      name: teamInitName.value.trim(),
      ownerLabel: currentProject.value?.name || "local-operator",
    });
    teamInitName.value = "";
    await loadSummary();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function handleTeamAddMember() {
  if (!currentProjectPath.value || !teamMemberLabel.value.trim()) return;
  acting.value = true;
  error.value = null;
  try {
    teamWs.value = await addTeamMember(currentProjectPath.value, {
      label: teamMemberLabel.value.trim(),
      meshPeerId: teamMemberPeer.value.trim() || undefined,
      role: "member",
    });
    teamMemberLabel.value = "";
    teamMemberPeer.value = "";
    await loadSummary();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function handleTeamRemoveMember(memberId: string) {
  if (!currentProjectPath.value || !memberId) return;
  acting.value = true;
  error.value = null;
  try {
    teamWs.value = await removeTeamMember(currentProjectPath.value, memberId);
    await loadSummary();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function handleTrustInit() {
  if (!currentProjectPath.value) return;
  if (!teamWs.value) {
    error.value = "Initialize a team workspace first (Team tab).";
    selectTab("team");
    return;
  }
  acting.value = true;
  error.value = null;
  try {
    trustPolicy.value = await initTeamTrustPolicy(currentProjectPath.value);
    trustAudit.value = await listTeamTrustAudit(currentProjectPath.value, 20);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function handleTrustToggleRemote() {
  if (!currentProjectPath.value || !trustPolicy.value) return;
  acting.value = true;
  error.value = null;
  try {
    trustPolicy.value = await setTeamTrustRemoteQuery(
      currentProjectPath.value,
      !trustPolicy.value.remoteQueryEnabled,
    );
    trustAudit.value = await listTeamTrustAudit(currentProjectPath.value, 20);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function handleTrustSetMode(mode: string) {
  if (!currentProjectPath.value) return;
  acting.value = true;
  error.value = null;
  try {
    trustPolicy.value = await setTeamTrustQueryMode(currentProjectPath.value, mode);
    trustAudit.value = await listTeamTrustAudit(currentProjectPath.value, 20);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function handleTrustAllowAdd() {
  if (!currentProjectPath.value || !trustAllowPeer.value.trim()) return;
  acting.value = true;
  error.value = null;
  try {
    trustPolicy.value = await addTeamTrustAllowlist(currentProjectPath.value, {
      meshPeerId: trustAllowPeer.value.trim(),
    });
    trustAllowPeer.value = "";
    trustAudit.value = await listTeamTrustAudit(currentProjectPath.value, 20);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function handleTrustAllowRemove(entry: {
  memberId?: string;
  meshPeerId?: string;
}) {
  if (!currentProjectPath.value) return;
  if (!entry.memberId && !entry.meshPeerId) return;
  acting.value = true;
  error.value = null;
  try {
    trustPolicy.value = await removeTeamTrustAllowlist(currentProjectPath.value, {
      memberId: entry.memberId,
      meshPeerId: entry.meshPeerId,
    });
    trustAudit.value = await listTeamTrustAudit(currentProjectPath.value, 20);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

function trustPeerFromRoster() {
  if (peers.value.length === 1 && !trustAllowPeer.value.trim()) {
    trustAllowPeer.value = peers.value[0].peerId;
  }
}

async function refreshChatMessages() {
  if (!currentProjectPath.value) return;
  try {
    // Fetch recent local store, then filter by host:port and/or discovered peer id
    // (inbound LAN messages are keyed by from_peer_id; outbound by host:port).
    const all = await lanChatList(currentProjectPath.value, undefined, 200);
    const addr = chatPeerTo.value.trim();
    if (!addr) {
      chatMessages.value = all;
      return;
    }
    const peerId = presenceFor(addr)?.health?.peerId;
    chatMessages.value = all.filter(
      (m) =>
        m.peerKey === addr ||
        (!!peerId &&
          (m.peerKey === peerId || m.message.fromPeerId === peerId)),
    );
  } catch {
    chatMessages.value = [];
  }
}

async function handleChatSend() {
  if (!currentProjectPath.value || !chatPeerTo.value.trim() || !chatText.value.trim()) {
    return;
  }
  acting.value = true;
  error.value = null;
  try {
    await lanChatSend(
      currentProjectPath.value,
      chatPeerTo.value.trim(),
      chatText.value.trim(),
      { fromLabel: currentProject.value?.name || undefined },
    );
    chatText.value = "";
    await refreshChatMessages();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    acting.value = false;
  }
}

async function refreshLanApproved() {
  if (!currentProjectPath.value) return;
  try {
    lanApprovedPackages.value = await lanListApprovedPackages(
      currentProjectPath.value,
    );
  } catch {
    lanApprovedPackages.value = [];
  }
}

function selectLanPeer(peer: LanPeerInfo) {
  lanSendTo.value = peer.address;
  lanAskTo.value = peer.address;
  chatPeerTo.value = peer.address;
  lanManualProbe.value = peer.address;
}

function selectMeshPeer(p: MeshPeerRecord) {
  meshPeer.value = p.peerId;
  if (p.lanAddress) {
    chatPeerTo.value = p.lanAddress;
    lanAskTo.value = p.lanAddress;
    lanSendTo.value = p.lanAddress;
    void refreshLanPresence();
  }
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
  if (tab.value !== "lan" && tab.value !== "chat") {
    stopPresencePolling();
  }
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
  trustAudit.value = [];
  connectors.value = [];
  orgGraph.value = null;
  pilotPack.value = null;
  rcPack.value = null;
  lanPresenceByAddress.value = {};
  chatMessages.value = [];
  stopPresencePolling();
  loadTab();
});

onMounted(() => {
  loadTab();
});

onUnmounted(() => {
  stopPresencePolling();
});
</script>

<template>
  <div class="cont animate-fade-in">
    <header class="cont__head">
      <div class="cont__head-main">
        <p class="text-caption uppercase tracking-[0.18em] text-muted">Runtime</p>
        <h1 class="cont__title">Pending &amp; LAN</h1>
        <p v-if="hasProject && statusLine" class="cont__meta">{{ statusLine }}</p>
        <p v-else-if="!hasProject" class="cont__meta">Open a project to see pending work and local peers.</p>
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
      No project selected. Choose a project in the sidebar to see pending work and LAN peers.
    </div>

    <template v-else>
      <nav class="om-nav" aria-label="Pending and LAN sections">
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

      <ol
        v-if="showLanDogfoodGuide"
        class="cont__flow"
        aria-label="LAN dogfood steps"
      >
        <li
          v-for="(step, i) in lanDogfoodSteps"
          :key="step.id"
          class="cont__flow-step"
          :class="{
            'is-active': tab === step.id,
            'is-done': lanDogfoodDone[step.id],
          }"
        >
          <button
            type="button"
            class="cont__flow-btn"
            :title="step.hint"
            @click="selectTab(step.id)"
          >
            <span class="cont__flow-num">{{ i + 1 }}</span>
            <span class="cont__flow-label">{{ step.label }}</span>
          </button>
          <span
            v-if="i < lanDogfoodSteps.length - 1"
            class="cont__flow-sep"
            aria-hidden="true"
          >→</span>
        </li>
      </ol>
      <p v-if="showLanDogfoodGuide" class="cont__flow-hint">
        Trusted-LAN alpha (no WAN / no cloud relay). Local registry + same-network HTTP —
        not finished-product encryption.
      </p>

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
            <p class="cont__empty-title">Nothing needs attention</p>
            <p class="cont__empty-body">
              When Chat records a verify or a pending question, it shows up here.
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
          <h3 class="section-label">Register peer</h3>
          <p class="text-[12px] text-muted">
            Step 1 — local peer registry. Optional LAN <code class="code">host:port</code>
            enables green-dot presence and Chat. Next: Team → Trust → start LAN listener.
          </p>
          <input
            v-model="meshAddLabel"
            class="input-sm w-full"
            placeholder="Label (e.g. Yo)"
          />
          <input
            v-model="meshAddLan"
            class="input-sm w-full"
            placeholder="LAN host:port (optional, e.g. 192.168.1.12:41778)"
          />
          <input
            v-model="meshAddNotes"
            class="input-sm w-full"
            placeholder="Notes (optional)"
          />
          <button
            type="button"
            class="btn-primary"
            :disabled="acting || !meshAddLabel.trim()"
            @click="handleMeshAddPeer"
          >
            Add peer
          </button>
        </div>
        <div class="workbench-card p-4 space-y-3">
          <h3 class="section-label">Peers ({{ peers.length }})</h3>
          <div v-if="peers.length === 0" class="cont__empty">
            <p class="cont__empty-title">No peers yet</p>
            <p class="cont__empty-body">
              Add a peer above (include LAN host:port if you already know it), then continue to
              <button type="button" class="cont__inline-link" @click="selectTab('team')">Team</button>.
            </p>
          </div>
          <div
            v-for="p in peers"
            :key="p.peerId"
            class="rounded-lg p-3 text-[12px] cursor-pointer"
            style="background: var(--surface-2); border: 1px solid var(--border)"
            @click="selectMeshPeer(p)"
          >
            <div class="flex items-center gap-2">
              <span
                v-if="p.lanAddress"
                class="presence-dot"
                :class="'presence-dot--' + (presenceFor(p.lanAddress)?.state || 'unknown')"
                :title="presenceLabel(presenceFor(p.lanAddress)?.state)"
              />
              <div class="font-medium text-[13px]">{{ p.label }}</div>
            </div>
            <div class="text-muted">id={{ p.peerId }}</div>
            <div v-if="p.remoteWorkspaceId" class="text-muted">
              workspace={{ p.remoteWorkspaceId }}
            </div>
            <div v-if="p.lanAddress" class="text-muted">
              lan={{ p.lanAddress }}
              <span v-if="presenceFor(p.lanAddress)" class="badge">
                {{ presenceLabel(presenceFor(p.lanAddress)?.state) }}
              </span>
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
            <ChatMessageContent
              class="continuity-answer-md"
              :text="meshAnswer.answerText"
            />
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
            Step 4 — start the listener so peers can discover / ask / chat here.
            Trusted-LAN alpha: UDP + HTTP. macOS may prompt for firewall on first bind.
            VPN or loopback-only networks often break UDP — paste the peer’s
            <code class="text-[11px]">host:port</code> manually. No WAN.
          </p>
          <div v-if="lanStatus?.running" class="flex items-center gap-2 text-[13px]">
            <CheckCircle2 class="h-4 w-4" style="color: var(--accent-green)" />
            <span class="font-medium">Listening</span>
            <span class="badge">{{ lanStatus.httpHost }}:{{ lanStatus.httpPort }}</span>
          </div>
          <div v-else class="text-[12px] text-muted">
            Listener is stopped
            <span v-if="lanStatus?.note"> — {{ lanStatus.note }}</span>
          </div>
          <div class="flex items-center gap-2 flex-wrap">
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
          <div v-if="lanStatus?.running && lanStatus?.note" class="text-[11px] text-muted">
            {{ lanStatus.note }}
          </div>
        </div>
        <div class="workbench-card p-4 space-y-3">
          <div class="flex items-center justify-between gap-2">
            <h3 class="section-label">Peers ({{ lanPeers.length }})</h3>
            <span v-if="presenceProbing" class="text-[11px] text-muted">probing…</span>
          </div>
          <div v-if="lanPeers.length === 0" class="text-[12px] text-muted space-y-2">
            <p>No peers discovered this scan.</p>
            <ul class="list-disc pl-4 space-y-1">
              <li>Start a listener on the other machine (Continuity → LAN).</li>
              <li>
                If you’re on VPN / different subnet / loopback-only, skip UDP and
                probe a manual <code class="text-[11px]">host:httpPort</code>
                below (default HTTP port 41778).
              </li>
              <li>
                Peer needs an API key in Settings — otherwise Ask returns
                <code class="text-[11px]">missing_api_key</code>.
              </li>
            </ul>
          </div>
          <p v-else class="text-[11px] text-muted">
            Green = live health. Amber = seen recently but health failed. Gray =
            unreachable. Probes <code class="text-[11px]">GET /v1/health</code>
            every ~8s while this tab is open.
          </p>
          <div
            v-for="p in lanPeers"
            :key="p.peerId + p.address"
            class="rounded-lg p-3 text-[12px] cursor-pointer"
            style="background: var(--surface-2); border: 1px solid var(--border)"
            @click="selectLanPeer(p)"
          >
            <div class="flex items-center gap-2">
              <span
                class="presence-dot"
                :class="'presence-dot--' + (presenceFor(p.address)?.state || 'unknown')"
                :title="presenceLabel(presenceFor(p.address)?.state)"
              />
              <div class="font-medium text-[13px]">{{ p.ownerLabel }}</div>
              <span class="badge">{{
                presenceLabel(presenceFor(p.address)?.state)
              }}</span>
              <span
                v-if="presenceFor(p.address)?.latencyMs != null"
                class="text-muted text-[11px]"
              >
                {{ presenceFor(p.address)?.latencyMs }}ms
              </span>
            </div>
            <div class="text-muted">id={{ p.peerId }} · {{ p.address }}</div>
            <div class="text-muted text-[11px]">
              discovered {{ p.lastSeenAt }}
              <span v-if="presenceFor(p.address)?.probedAt">
                · probed {{ presenceFor(p.address)?.probedAt }}
              </span>
            </div>
          </div>
        </div>
        <div class="workbench-card p-4 space-y-3">
          <h3 class="section-label">Manual host:port probe</h3>
          <p class="text-[12px] text-muted">
            Probe any LAN peer without UDP discovery (VPN / other subnet).
          </p>
          <div class="flex items-center gap-2 flex-wrap">
            <input
              v-model="lanManualProbe"
              class="input-sm flex-1 min-w-[12rem]"
              placeholder="192.168.1.20:41778"
            />
            <button
              type="button"
              class="btn-primary"
              :disabled="acting || !lanManualProbe.trim()"
              @click="handleManualProbe"
            >
              Probe
            </button>
          </div>
          <div
            v-if="lanManualProbeResult"
            class="flex items-center gap-2 text-[12px]"
          >
            <span
              class="presence-dot"
              :class="'presence-dot--' + lanManualProbeResult.state"
            />
            <span class="badge">{{ lanManualProbeResult.state }}</span>
            <span class="text-muted">{{ lanManualProbeResult.address }}</span>
            <span v-if="lanManualProbeResult.health" class="text-muted">
              · {{ lanManualProbeResult.health.ownerLabel }}
            </span>
          </div>
        </div>
        <div class="workbench-card p-4 space-y-3">
          <h3 class="section-label">Send approved package</h3>
          <p class="text-[12px] text-muted">
            Pack / approve stay CLI-first. Desktop only sends packages already
            under <code class="text-[11px]">relay/approved/</code>.
          </p>
          <pre
            class="text-[11px] whitespace-pre-wrap font-mono rounded p-2"
            style="background: var(--surface-3)"
          >{{ packCliHint }}</pre>
          <button
            type="button"
            class="btn-ghost"
            @click="copyText(packCliHint)"
          >
            Copy pack+approve commands
          </button>
          <div v-if="lanApprovedPackages.length" class="space-y-1">
            <div class="text-[12px] font-medium">Approved on this project</div>
            <button
              v-for="id in lanApprovedPackages"
              :key="id"
              type="button"
              class="btn-ghost text-[12px] block"
              @click="lanSendId = id"
            >
              {{ id }}
            </button>
          </div>
          <div v-else class="text-[12px] text-muted">
            No approved packages yet — run the CLI helpers above.
          </div>
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
          <h3 class="section-label">Ask peer (live Agent Engine)</h3>
          <p class="text-[12px] text-muted">
            Read-only live answer from the peer’s Agent Engine (their configured
            provider/key). Not LocalScaffold. Does not write their project files
            or your ledger.
          </p>
          <input
            v-model="lanAskTo"
            class="input-sm w-full"
            placeholder="Peer host:port (e.g. 192.168.1.20:41778)"
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
              {{ acting ? "Asking…" : "Ask peer" }}
            </button>
          </div>
          <div
            v-if="lanAskAnswer"
            class="rounded-lg p-3 space-y-2"
            style="background: var(--surface-2); border: 1px solid var(--border)"
          >
            <div class="flex items-center gap-2 text-[12px] flex-wrap">
              <span class="badge">{{ lanAskAnswer.peerLabel }}</span>
              <span v-if="lanAskAnswer.readOnly" class="badge">read-only</span>
              <span class="badge">live</span>
              <span v-if="lanAskAnswer.refused" class="badge">refused</span>
            </div>
            <div class="text-[12px] text-muted">{{ lanAskAnswer.freshness.statement }}</div>
            <ChatMessageContent
              class="continuity-answer-md"
              :text="lanAskAnswer.answerText"
            />
          </div>
        </div>
      </div>

      <!-- Team Chat (LAN text MVP) -->
      <div v-else-if="tab === 'chat'" class="space-y-3">
        <div class="workbench-card p-4 space-y-3">
          <h3 class="section-label">Team Chat (LAN)</h3>
          <p class="text-[12px] text-muted">
            Text peer delivery over LAN HTTP
            (<code class="code">POST /v1/chat/message</code>. Not WhatsApp —
            local/LAN text only, trusted-LAN alpha (no E2E crypto product claim).
            Both sides need Continuity → LAN listener running.
          </p>
          <div class="flex flex-wrap gap-2">
            <button
              v-for="p in lanPeers"
              :key="'chat-' + p.address"
              type="button"
              class="btn-ghost text-[12px] inline-flex items-center gap-1.5"
              @click="chatPeerTo = p.address; refreshChatMessages()"
            >
              <span
                class="presence-dot"
                :class="'presence-dot--' + (presenceFor(p.address)?.state || 'unknown')"
              />
              {{ p.ownerLabel }}
            </button>
          </div>
          <input
            v-model="chatPeerTo"
            class="input-sm w-full"
            placeholder="Peer host:port"
            @change="refreshChatMessages"
          />
          <div
            class="rounded-lg p-3 space-y-2 max-h-64 overflow-y-auto"
            style="background: var(--surface-2); border: 1px solid var(--border)"
          >
            <div v-if="chatMessages.length === 0" class="text-[12px] text-muted">
              No messages in this thread yet.
            </div>
            <div
              v-for="m in chatMessages"
              :key="m.message.messageId + m.storedAt"
              class="text-[12px]"
              :class="m.direction === 'outbound' ? 'text-right' : ''"
            >
              <div class="text-[11px] text-muted">
                {{ m.message.fromLabel }}
                · {{ m.direction }}
                · {{ m.message.sentAt }}
              </div>
              <div
                class="inline-block rounded-lg px-2.5 py-1.5 mt-0.5 text-left max-w-full"
                style="background: var(--surface-3)"
              >
                <ChatMessageContent
                  class="continuity-answer-md"
                  :text="m.message.text"
                />
              </div>
            </div>
          </div>
          <textarea
            v-model="chatText"
            rows="2"
            class="input-area w-full"
            placeholder="Message…"
          />
          <div class="flex items-center gap-2">
            <button
              type="button"
              class="btn-primary"
              :disabled="acting || !chatPeerTo.trim() || !chatText.trim()"
              @click="handleChatSend"
            >
              {{ acting ? "Sending…" : "Send" }}
            </button>
            <button type="button" class="btn-ghost" @click="refreshChatMessages">
              Refresh thread
            </button>
          </div>
        </div>
      </div>

      <!-- Relay -->
      <div v-else-if="tab === 'relay'" class="space-y-3">
        <div class="workbench-card p-4 space-y-3">
          <h3 class="section-label">Audit trail ({{ audit.length }})</h3>
          <p class="text-[12px] text-muted">
            Pack / approve / send / receive remain CLI-first. This view is
            read-only audit. Relay packages are filesystem/LAN transfer — not a
            WAN cloud mesh.
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

      <!-- Continuity Proxy (live Agent Engine) -->
      <div v-else-if="tab === 'online-proxy'" class="space-y-3">
        <div class="workbench-card p-4 space-y-3">
          <template v-if="!onlineConfig">
            <p class="text-[13px] text-muted">
              Continuity Proxy is not initialized for this project. Init stores
              local config; Ask uses your Agent Engine provider (Settings → API
              key) — not a remote cloud teammate. WAN / other-Wi‑Fi peer reach
              is not available without a real relay you host; mesh peers remain
              LAN or file-envelope based today.
            </p>
            <button
              type="button"
              class="btn-primary"
              :disabled="acting"
              @click="handleInitProxy"
            >
              Initialize Continuity Proxy
            </button>
          </template>
          <template v-else>
            <div class="flex items-center gap-2 text-[13px] flex-wrap">
              <CheckCircle2 class="h-4 w-4" style="color: var(--accent-green)" />
              <span class="font-medium">{{ onlineConfig.proxyId }}</span>
              <span class="badge">{{ onlineConfig.mode }}</span>
              <span class="badge">live ask = Agent Engine</span>
            </div>
            <div class="text-[12px] text-muted">
              owner={{ onlineConfig.ownerLabel }} · relay-received={{
                onlineConfig.useRelayReceived
              }}
              · default tier={{ onlineConfig.defaultFreshnessTier }}
            </div>
            <p class="text-[12px] text-muted">
              Mode label may still say local-scaffold (config legacy). Answers
              are live LLM via Agent Engine with freshness injected as context.
              Critical tier still refuses when evidence is insufficient.
            </p>
            <div class="space-y-2 pt-2">
              <label class="text-[12px] font-medium">Live ask</label>
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
                  {{ acting ? "Asking…" : "Live ask" }}
                </button>
              </div>
            </div>
            <div
              v-if="lastAnswer"
              class="rounded-lg p-3 space-y-2 mt-2"
              style="background: var(--surface-2); border: 1px solid var(--border)"
            >
              <div class="flex items-center gap-2 text-[12px] flex-wrap">
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
                <span v-if="lastAnswer.liveEngine" class="badge">live</span>
                <span v-if="lastAnswer.refused" class="badge">refused</span>
              </div>
              <div
                class="text-[12px] rounded p-2"
                style="background: var(--surface-3); color: var(--muted-foreground)"
              >
                {{ lastAnswer.freshness.statement }}
              </div>
              <ChatMessageContent
                class="continuity-answer-md"
                :text="lastAnswer.answerText"
              />
            </div>
          </template>
        </div>
      </div>

      <!-- Team (0.1.15) -->
      <div v-else-if="tab === 'team'" class="space-y-3">
        <template v-if="!teamWs">
          <div class="workbench-card p-5 space-y-3">
            <h3 class="text-[14px] font-semibold">Initialize team</h3>
            <p class="text-[12px] text-muted">
              Step 2 — local team registry for this project. Not multi-tenant cloud admin.
              Cloud sync remains CLI scaffold only (<code class="code">team cloud sync-scaffold</code>).
            </p>
            <div
              v-if="peers.length === 0"
              class="text-[12px] rounded p-3"
              style="background: var(--surface-2); border: 1px solid var(--border)"
            >
              Tip: register a peer first
              (<button type="button" class="cont__inline-link" @click="selectTab('mesh')">Peers</button>)
              so you can link members afterward.
            </div>
            <input
              v-model="teamInitName"
              class="input-sm w-full"
              placeholder="Team display name"
            />
            <button
              type="button"
              class="btn-primary"
              :disabled="acting || !teamInitName.trim()"
              @click="handleTeamInit"
            >
              Initialize team
            </button>
          </div>
        </template>
        <template v-else>
          <div class="workbench-card p-5 space-y-3">
            <div class="flex items-start justify-between gap-3 flex-wrap">
              <div>
                <h3 class="text-[14px] font-semibold">{{ teamWs.displayName }}</h3>
                <p class="text-[12px] text-muted mt-0.5">
                  {{ teamWs.members?.length ?? 0 }} members · local registry
                </p>
              </div>
              <button
                v-if="!trustPolicy"
                type="button"
                class="btn-ghost"
                @click="selectTab('trust')"
              >
                Next: Trust →
              </button>
            </div>
            <ul class="space-y-2 text-[12px]">
              <li
                v-for="m in teamWs.members"
                :key="m.memberId"
                class="flex items-center gap-2 flex-wrap rounded-lg p-2.5"
                style="background: var(--surface-2); border: 1px solid var(--border)"
              >
                <span
                  v-if="m.meshPeerId && peerById(m.meshPeerId)?.lanAddress"
                  class="presence-dot"
                  :class="
                    'presence-dot--' +
                    (presenceForPeerId(m.meshPeerId)?.state || 'unknown')
                  "
                  :title="presenceLabel(presenceForPeerId(m.meshPeerId)?.state)"
                />
                <span class="font-medium text-[13px]">{{ m.label }}</span>
                <span class="badge">{{ m.role }}</span>
                <span
                  v-if="m.meshPeerId"
                  class="text-muted"
                >
                  {{ peerById(m.meshPeerId)?.label || m.meshPeerId }}
                  <template v-if="peerById(m.meshPeerId)?.lanAddress">
                    · {{ peerById(m.meshPeerId)?.lanAddress }}
                  </template>
                </span>
                <span v-else class="text-muted">no linked peer</span>
                <button
                  v-if="canRemoveMember(m.memberId, m.role)"
                  type="button"
                  class="btn-ghost ml-auto"
                  :disabled="acting"
                  @click="handleTeamRemoveMember(m.memberId)"
                >
                  Remove
                </button>
              </li>
            </ul>
          </div>
          <div class="workbench-card p-5 space-y-3">
            <h3 class="section-label">Add member</h3>
            <p class="text-[12px] text-muted">
              Link a mesh peer so Trust allowlist and LAN presence stay aligned.
            </p>
            <input
              v-model="teamMemberLabel"
              class="input-sm w-full"
              placeholder="Member label"
            />
            <select v-model="teamMemberPeer" class="input-sm w-full">
              <option value="">No linked peer</option>
              <option
                v-for="p in peers"
                :key="p.peerId"
                :value="p.peerId"
              >
                {{ p.label }} ({{ p.peerId }}){{
                  p.lanAddress ? ` · ${p.lanAddress}` : ""
                }}
              </option>
            </select>
            <button
              type="button"
              class="btn-primary"
              :disabled="acting || !teamMemberLabel.trim()"
              @click="handleTeamAddMember"
            >
              Add member
            </button>
            <p v-if="peers.length === 0" class="text-[12px] text-muted">
              No peers to link —
              <button type="button" class="cont__inline-link" @click="selectTab('mesh')">
                Add peer
              </button>
              first.
            </p>
          </div>
        </template>
      </div>

      <!-- Trust (0.1.17) -->
      <div v-else-if="tab === 'trust'" class="space-y-3">
        <template v-if="!teamWs">
          <div class="workbench-card p-5 space-y-3">
            <h3 class="text-[14px] font-semibold">Team required first</h3>
            <p class="text-[12px] text-muted">
              Trust policy is scoped to a team workspace. Init Team, then return here.
            </p>
            <button type="button" class="btn-primary" @click="selectTab('team')">
              Go to Team → Init
            </button>
          </div>
        </template>
        <template v-else-if="!trustPolicy">
          <div class="workbench-card p-5 space-y-3">
            <h3 class="text-[14px] font-semibold">Initialize trust</h3>
            <p class="text-[12px] text-muted">
              Step 3 — local policy for remote query gating on LAN/mesh.
              Secrets stay fail-closed. This is <em>not</em> finished-product E2E mesh crypto.
            </p>
            <button
              type="button"
              class="btn-primary"
              :disabled="acting"
              @click="handleTrustInit"
            >
              Initialize trust policy
            </button>
          </div>
        </template>
        <template v-else>
          <div class="workbench-card p-5 space-y-3 text-[12px]">
            <div class="flex items-start justify-between gap-3 flex-wrap">
              <div>
                <h3 class="text-[14px] font-semibold">Trust this peer on LAN</h3>
                <p class="text-muted mt-0.5">
                  team {{ trustPolicy.teamId }} · policy gating only
                </p>
              </div>
              <button type="button" class="btn-ghost" @click="selectTab('lan')">
                Next: LAN →
              </button>
            </div>
            <div class="flex flex-wrap items-center gap-2">
              <span class="badge">
                remote query
                {{ trustPolicy.remoteQueryEnabled ? "on" : "off" }}
              </span>
              <span class="badge">mode {{ trustPolicy.queryAllowlistMode }}</span>
              <span class="badge">
                allowlist {{ trustPolicy.queryAllowlist?.length ?? 0 }}
              </span>
              <span class="badge">secrets fail-closed</span>
            </div>
            <div class="flex flex-wrap gap-2 pt-1">
              <button
                type="button"
                class="btn-ghost"
                :disabled="acting"
                @click="handleTrustToggleRemote"
              >
                {{
                  trustPolicy.remoteQueryEnabled
                    ? "Disable remote query"
                    : "Enable remote query"
                }}
              </button>
              <button
                type="button"
                class="btn-ghost"
                :class="{
                  'is-mode-active': trustPolicy.queryAllowlistMode === 'allow-all',
                }"
                :disabled="acting"
                @click="handleTrustSetMode('allow-all')"
              >
                allow-all
              </button>
              <button
                type="button"
                class="btn-ghost"
                :class="{
                  'is-mode-active':
                    trustPolicy.queryAllowlistMode === 'allowlist-only',
                }"
                :disabled="acting"
                @click="handleTrustSetMode('allowlist-only')"
              >
                allowlist-only
              </button>
              <button
                type="button"
                class="btn-ghost"
                :class="{
                  'is-mode-active': trustPolicy.queryAllowlistMode === 'deny-all',
                }"
                :disabled="acting"
                @click="handleTrustSetMode('deny-all')"
              >
                deny-all
              </button>
            </div>
            <p class="text-muted">
              For LAN dogfood: enable remote query → set <strong>allowlist-only</strong> →
              add the peer below.
            </p>
          </div>
          <div class="workbench-card p-5 space-y-3">
            <h3 class="section-label">Allowlist peer</h3>
            <p class="text-[12px] text-muted">
              When mode is allowlist-only, only listed mesh peers may be queried.
            </p>
            <select
              v-model="trustAllowPeer"
              class="input-sm w-full"
              @focus="trustPeerFromRoster"
            >
              <option value="">Select a registered peer…</option>
              <option
                v-for="p in peers"
                :key="p.peerId"
                :value="p.peerId"
              >
                {{ p.label }} ({{ p.peerId }}){{
                  p.lanAddress ? ` · ${p.lanAddress}` : ""
                }}
              </option>
            </select>
            <input
              v-model="trustAllowPeer"
              class="input-sm w-full"
              placeholder="Or paste mesh peer id"
            />
            <button
              type="button"
              class="btn-primary"
              :disabled="acting || !trustAllowPeer.trim()"
              @click="handleTrustAllowAdd"
            >
              Trust this peer
            </button>
            <p v-if="peers.length === 0" class="text-[12px] text-muted">
              No peers registered —
              <button type="button" class="cont__inline-link" @click="selectTab('mesh')">
                Add peer
              </button>
              first.
            </p>
            <ul
              v-if="trustPolicy.queryAllowlist?.length"
              class="space-y-2 text-[12px]"
            >
              <li
                v-for="(e, i) in trustPolicy.queryAllowlist"
                :key="i"
                class="flex items-center gap-2 flex-wrap rounded-lg p-2.5"
                style="background: var(--surface-2); border: 1px solid var(--border)"
              >
                <span
                  v-if="e.meshPeerId && peerById(e.meshPeerId)?.lanAddress"
                  class="presence-dot"
                  :class="
                    'presence-dot--' +
                    (presenceForPeerId(e.meshPeerId)?.state || 'unknown')
                  "
                  :title="presenceLabel(presenceForPeerId(e.meshPeerId)?.state)"
                />
                <span class="font-medium">
                  {{ peerById(e.meshPeerId)?.label || e.meshPeerId || e.memberId || "entry" }}
                </span>
                <span class="text-muted">
                  peer={{ e.meshPeerId || "-" }}
                  <template v-if="e.memberId"> · member={{ e.memberId }}</template>
                </span>
                <button
                  type="button"
                  class="btn-ghost ml-auto"
                  :disabled="acting"
                  @click="handleTrustAllowRemove(e)"
                >
                  Remove
                </button>
              </li>
            </ul>
            <p
              v-else
              class="text-[12px] text-muted"
            >
              Allowlist empty — add a peer to trust on LAN, or use allow-all mode.
            </p>
          </div>
          <div class="workbench-card p-5 space-y-2">
            <h3 class="section-label">Admin audit</h3>
            <div v-if="trustAudit.length === 0" class="text-[12px] text-muted">
              No audit events yet. Init / mode / allowlist changes appear here.
            </div>
            <div
              v-for="ev in trustAudit"
              :key="ev.eventId"
              class="text-[12px] rounded p-2"
              style="background: var(--surface-2)"
            >
              <span class="badge">{{ ev.action }}</span>
              {{ ev.detail }}
              <div class="text-muted text-[11px]">{{ ev.at }} · {{ ev.actorMemberId }}</div>
            </div>
          </div>
        </template>
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

.cont__flow {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.15rem 0.25rem;
}

.cont__flow-step {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
}

.cont__flow-btn {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.3rem 0.55rem;
  border-radius: 999px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--muted-foreground);
  font-size: 0.72rem;
  font-weight: 600;
  cursor: pointer;
}

.cont__flow-step.is-active .cont__flow-btn {
  border-color: color-mix(in srgb, var(--accent-blue) 55%, var(--border));
  color: var(--foreground);
  background: color-mix(in srgb, var(--accent-blue) 12%, var(--surface-2));
}

.cont__flow-step.is-done .cont__flow-num {
  background: var(--accent-green, #22c55e);
  color: #fff;
}

.cont__flow-num {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.15rem;
  height: 1.15rem;
  border-radius: 999px;
  font-size: 0.65rem;
  background: var(--surface-3);
  color: var(--muted-foreground);
}

.cont__flow-sep {
  color: var(--muted-foreground);
  opacity: 0.55;
  font-size: 0.75rem;
  padding: 0 0.1rem;
}

.cont__flow-hint {
  margin: -0.35rem 0 0;
  font-size: 0.72rem;
  color: var(--muted-foreground);
}

.cont__inline-link {
  display: inline;
  padding: 0;
  border: none;
  background: none;
  color: var(--accent-blue);
  font: inherit;
  font-weight: 600;
  cursor: pointer;
  text-decoration: underline;
  text-underline-offset: 2px;
}

.btn-ghost.is-mode-active {
  border-color: color-mix(in srgb, var(--accent-blue) 50%, var(--border));
  background: color-mix(in srgb, var(--accent-blue) 10%, transparent);
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
.presence-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
  background: var(--muted-foreground);
  opacity: 0.55;
}
.presence-dot--live {
  background: var(--accent-green, #22c55e);
  opacity: 1;
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent-green, #22c55e) 25%, transparent);
}
.presence-dot--stale {
  background: var(--accent-amber, #f59e0b);
  opacity: 1;
}
.presence-dot--unreachable,
.presence-dot--unknown {
  background: var(--muted-foreground);
  opacity: 0.45;
}

/* Match Continuity card density; ChatMessageContent already uses theme tokens. */
.continuity-answer-md :deep(.chat-prose) {
  font-size: 12px;
  line-height: 1.55;
  color: var(--foreground);
}
</style>
