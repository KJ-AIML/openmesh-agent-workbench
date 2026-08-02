// OpenMesh Continuity Desktop Client — Dev Track 0.1.13
// Thin wrappers around Tauri IPC for pending / digest / mesh / relay / online-proxy.

import { invoke } from "@tauri-apps/api/core";

export interface PendingQuestionItem {
  id: string;
  summary: string;
  source: "proxy-pending" | "continuity-attention" | "unresolved-signal" | string;
  sourceId: string;
  status: string;
  severity: string;
  createdAt: string;
  reason: string;
  risk?: string;
  resolvedAuthority?: string;
  evidenceRefs?: unknown[];
}

export interface PendingQuestionsView {
  workspaceId: string;
  generatedAt: string;
  protocolVersion: string;
  items: PendingQuestionItem[];
  openCount: number;
  sourceCounts: {
    proxyPending: number;
    continuityAttention: number;
    unresolvedSignal: number;
  };
  limitations: string[];
}

export interface ReturnDigest {
  workspaceId: string;
  generatedAt: string;
  protocolVersion: string;
  window: { since: string; until: string };
  summary: string;
  needsMe: PendingQuestionItem[];
  whatIMissed: {
    completed: unknown[];
    changed: unknown[];
    blocked: unknown[];
    decided: unknown[];
    needsAttention: unknown[];
    stillOpen: unknown[];
  };
  catchUpSummary: string;
  handoffs: Array<{
    handoffId: string;
    status: string;
    recipientLabel: string;
    createdAt: string;
    updatedAt: string;
  }>;
  evidenceRefs: unknown[];
  limitations: string[];
}

export interface MeshPeerRecord {
  protocolVersion: string;
  peerId: string;
  label: string;
  proxyProfileId?: string;
  remoteWorkspaceId?: string;
  notes?: string;
  createdAt: string;
  updatedAt: string;
}

export interface MeshEnvelopeSummary {
  envelopeId: string;
  mailbox: "inbox" | "outbox" | string;
  fromPeer: { label: string; proxyProfileId?: string; workspaceId?: string };
  toPeer?: { label: string; proxyProfileId?: string; workspaceId?: string };
  generatedAt: string;
  evidenceItemCount: number;
  handoffIdCount: number;
  limitationCount: number;
  attributedTo: string;
}

export interface RelayAuditEvent {
  protocolVersion: string;
  eventId: string;
  packageId: string;
  kind: string;
  at: string;
  actorLabel?: string;
  detail: string;
  sensitivityMax?: string;
}

export interface OnlineProxyConfig {
  protocolVersion: string;
  proxyId: string;
  workspaceId: string;
  ownerLabel: string;
  mode: "local-scaffold" | "cloud-scaffold" | string;
  defaultFreshnessTier: string;
  useRelayReceived: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface OnlineProxyAnswer {
  protocolVersion: string;
  answerId: string;
  proxyId: string;
  workspaceId: string;
  question: string;
  answerText: string;
  generatedAt: string;
  freshness: {
    statement: string;
    evaluatedAt: string;
    tier: string;
    isSufficient: boolean;
    confidenceLabel: string;
    oldestEvidenceAgeSeconds: number;
    staleWarnings: string[];
    evidenceSourceIds: string[];
  };
  refused: boolean;
  mode: string;
}

export interface ContinuityHubSummary {
  openPendingCount: number;
  peerCount: number;
  envelopeCount: number;
  auditEventCount: number;
  onlineProxyInitialized: boolean;
}

export async function getContinuityHubSummary(
  projectPath: string,
): Promise<ContinuityHubSummary> {
  return invoke<ContinuityHubSummary>("continuity_hub_summary", { projectPath });
}

export async function getPendingQuestions(
  projectPath: string,
): Promise<PendingQuestionsView> {
  return invoke<PendingQuestionsView>("continuity_pending", { projectPath });
}

export async function getReturnDigest(
  projectPath: string,
  sinceHours?: number,
): Promise<ReturnDigest> {
  return invoke<ReturnDigest>("continuity_digest", {
    projectPath,
    sinceHours: sinceHours ?? null,
  });
}

export async function listMeshPeers(
  projectPath: string,
): Promise<MeshPeerRecord[]> {
  return invoke<MeshPeerRecord[]>("mesh_list_peers", { projectPath });
}

export async function listMeshEnvelopes(
  projectPath: string,
  mailbox?: "inbox" | "outbox" | "all",
): Promise<MeshEnvelopeSummary[]> {
  return invoke<MeshEnvelopeSummary[]>("mesh_list_envelopes", {
    projectPath,
    mailbox: mailbox && mailbox !== "all" ? mailbox : null,
  });
}

export interface MeshRemoteQueryAnswer {
  protocolVersion: string;
  queryId: string;
  peerId: string;
  peerLabel: string;
  question: string;
  answerText: string;
  generatedAt: string;
  readOnly: boolean;
  freshness: {
    statement: string;
    evaluatedAt: string;
    tier: string;
    isSufficient: boolean;
    confidenceLabel: string;
    oldestEvidenceAgeSeconds: number;
    staleWarnings: string[];
    evidenceSourceIds: string[];
  };
  refused: boolean;
  envelopeIds: string[];
  evidenceSummaries: string[];
  limitations: string[];
}

export async function queryMeshPeer(
  projectPath: string,
  peer: string,
  question: string,
  opts?: { tier?: string; queryId?: string; includeRelayReceived?: boolean },
): Promise<MeshRemoteQueryAnswer> {
  return invoke<MeshRemoteQueryAnswer>("mesh_query_peer", {
    projectPath,
    request: {
      peer,
      question,
      tier: opts?.tier,
      queryId: opts?.queryId,
      includeRelayReceived: opts?.includeRelayReceived,
    },
  });
}

export async function listRelayAudit(
  projectPath: string,
): Promise<RelayAuditEvent[]> {
  return invoke<RelayAuditEvent[]>("relay_list_audit", { projectPath });
}

export async function getOnlineProxyStatus(
  projectPath: string,
): Promise<OnlineProxyConfig | null> {
  return invoke<OnlineProxyConfig | null>("online_proxy_status", {
    projectPath,
  });
}

export async function initOnlineProxy(
  projectPath: string,
  opts?: {
    ownerLabel?: string;
    mode?: string;
    useRelayReceived?: boolean;
  },
): Promise<OnlineProxyConfig> {
  return invoke<OnlineProxyConfig>("online_proxy_init", {
    projectPath,
    request: {
      ownerLabel: opts?.ownerLabel,
      mode: opts?.mode,
      useRelayReceived: opts?.useRelayReceived,
    },
  });
}

export async function askOnlineProxy(
  projectPath: string,
  question: string,
  opts?: { tier?: string; answerId?: string },
): Promise<OnlineProxyAnswer> {
  return invoke<OnlineProxyAnswer>("online_proxy_ask", {
    projectPath,
    request: {
      question,
      tier: opts?.tier,
      answerId: opts?.answerId,
    },
  });
}

// ── Team / Trust / Connectors / Org (0.1.15–0.1.19) ─────────────────

export interface TeamWorkspaceView {
  protocolVersion: string;
  teamId: string;
  displayName: string;
  hostWorkspaceId: string;
  members: Array<{
    memberId: string;
    label: string;
    role: string;
    meshPeerId?: string;
    joinedAt: string;
  }>;
  createdAt: string;
  updatedAt: string;
  limitations: string[];
}

export interface TeamTrustPolicyView {
  protocolVersion: string;
  teamId: string;
  remoteQueryEnabled: boolean;
  queryAllowlistMode: string;
  queryAllowlist: unknown[];
  secretTopicsFailClosed: boolean;
  allowSecretExport: boolean;
  syncRequireSelective: boolean;
  adminMemberIds: string[];
  limitations: string[];
}

export interface ConnectorDescriptorView {
  protocolVersion: string;
  connectorId: string;
  kind: string;
  displayName: string;
  role: string;
  enabled: boolean;
  externalRef?: string;
  limitations: string[];
}

export interface OrgGraphView {
  protocolVersion: string;
  teamId: string;
  generatedAt: string;
  nodes: Array<{
    id: string;
    kind: string;
    label: string;
    evidence: string;
  }>;
  edges: Array<{
    from: string;
    to: string;
    kind: string;
    evidence: string;
  }>;
  limitations: string[];
}

export async function getTeamWorkspace(
  projectPath: string,
): Promise<TeamWorkspaceView | null> {
  return invoke<TeamWorkspaceView | null>("team_workspace_status", {
    projectPath,
  });
}

export async function getTeamTrustPolicy(
  projectPath: string,
): Promise<TeamTrustPolicyView | null> {
  return invoke<TeamTrustPolicyView | null>("team_trust_policy_status", {
    projectPath,
  });
}

export async function listConnectors(
  projectPath: string,
): Promise<ConnectorDescriptorView[]> {
  return invoke<ConnectorDescriptorView[]>("connector_list", { projectPath });
}

export async function getOrgGraph(
  projectPath: string,
): Promise<OrgGraphView | null> {
  return invoke<OrgGraphView | null>("org_graph_show", { projectPath });
}


export interface PilotPackView {
  protocolVersion: string;
  workspaceId: string;
  generatedAt: string;
  pilotReady: boolean;
  passCount: number;
  warnCount: number;
  failCount: number;
  checks: Array<{
    id: string;
    title: string;
    status: string;
    evidence: string;
    detail?: string;
  }>;
  threatNotes: Array<{ id: string; title: string; summary: string; residual: string }>;
  runbook: Array<{ id: string; title: string; commandOrAction: string; purpose: string }>;
  limitations: string[];
}

export async function getPilotStatus(projectPath: string): Promise<PilotPackView> {
  return invoke<PilotPackView>("pilot_status", { projectPath });
}
