<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { AlertCircle, Check, CircleDashed, Plus, RefreshCw, Save, ShieldCheck, Trash2, X } from "lucide-vue-next";
import {
  getBuiltInProxyManagementConfig,
  updateBuiltInProxyManagementConfig,
  type BuiltInProxyManagementConfig,
  type BuiltInProxyManagementUpstream,
} from "../lib/builtinProxyClient";

const config = ref<BuiltInProxyManagementConfig | null>(null);
const loading = ref(true);
const busy = ref(false);
const error = ref("");
const notice = ref("");
const editorOpen = ref(false);
const editing = ref<BuiltInProxyManagementUpstream | null>(null);
const aliasesDraft = ref("");
const fallbacksDraft = ref("");
const draft = ref({
  id: "",
  baseUrl: "",
  protocol: "open-ai-compatible" as BuiltInProxyManagementUpstream["protocol"],
  apiKey: "",
  models: "",
  priority: "0",
  accountId: "",
  oauthProvider: "",
  enabled: true,
});

const canEdit = computed(() => Boolean(config.value) && !loading.value && !busy.value);

function clearMessage() { error.value = ""; notice.value = ""; }

async function loadConfig() {
  loading.value = true;
  clearMessage();
  try {
    config.value = await getBuiltInProxyManagementConfig();
    syncModelRoutingDrafts(config.value);
  }
  catch (cause) { config.value = null; error.value = cause instanceof Error ? cause.message : "Unable to load OpenMesh proxy configuration."; }
  finally { loading.value = false; }
}

function syncModelRoutingDrafts(next: BuiltInProxyManagementConfig) {
  aliasesDraft.value = Object.entries(next.modelAliases).map(([alias, target]) => `${alias}=${target}`).join("\n");
  fallbacksDraft.value = Object.entries(next.modelFallbacks).map(([model, fallbacks]) => `${model}=${fallbacks.join(",")}`).join("\n");
}

function resetDraft() { draft.value = { id: "", baseUrl: "", protocol: "open-ai-compatible", apiKey: "", models: "", priority: "0", accountId: "", oauthProvider: "", enabled: true }; }
function modelsToText(upstream: BuiltInProxyManagementUpstream) { return upstream.models.map((model) => model.id).join("\n"); }
function openCreate() { clearMessage(); editing.value = null; resetDraft(); editorOpen.value = true; }
function openEdit(upstream: BuiltInProxyManagementUpstream) {
  clearMessage(); editing.value = upstream;
  draft.value = { id: upstream.id, baseUrl: upstream.baseUrl, protocol: upstream.protocol, apiKey: "", models: modelsToText(upstream), priority: String(upstream.priority), accountId: upstream.accountId ?? "", oauthProvider: upstream.oauthProvider ?? "", enabled: upstream.enabled };
  editorOpen.value = true;
}
function closeEditor() { if (!busy.value) { editorOpen.value = false; editing.value = null; } }
function forceCloseEditor() { editorOpen.value = false; editing.value = null; }

function parseModels(value: string) {
  const seen = new Set<string>();
  return value.split(/\r?\n/).map((item) => item.trim()).filter(Boolean).filter((id) => { const key = id.toLowerCase(); if (seen.has(key)) return false; seen.add(key); return true; }).map((id) => ({ id, ownedBy: "openmesh", capabilities: ["chat", "responses", "embeddings"] }));
}

function validateDraft() {
  const id = draft.value.id.trim();
  const baseUrl = draft.value.baseUrl.trim();
  if (!id) throw new Error("An upstream id is required.");
  if (!baseUrl) throw new Error("An upstream base URL is required.");
  let parsed: URL;
  try { parsed = new URL(baseUrl); } catch { throw new Error("The upstream base URL must be a valid HTTP(S) URL."); }
  if (!["http:", "https:"].includes(parsed.protocol) || parsed.username || parsed.password || parsed.search || parsed.hash) throw new Error("The upstream URL cannot contain credentials, query parameters, or fragments.");
  const priority = Number(draft.value.priority.trim() || "0");
  if (!Number.isSafeInteger(priority)) throw new Error("Priority must be a whole number.");
  const models = parseModels(draft.value.models);
  if (models.length === 0) throw new Error("Add at least one model to the upstream.");
  const oauthProvider = draft.value.oauthProvider.trim() || null;
  const accountId = draft.value.accountId.trim() || null;
  if (oauthProvider && !accountId) throw new Error("An OAuth-backed upstream needs an account id.");
  return { id, baseUrl, protocol: draft.value.protocol, ...(draft.value.apiKey.trim() ? { apiKey: draft.value.apiKey.trim() } : {}), enabled: draft.value.enabled, priority, accountId, oauthProvider, models };
}

async function applyUpstreams(upstreams: unknown[], message: string) {
  busy.value = true; clearMessage();
  try { config.value = await updateBuiltInProxyManagementConfig({ upstreams }); syncModelRoutingDrafts(config.value); notice.value = message; }
  catch (cause) { error.value = cause instanceof Error ? cause.message : "Unable to update OpenMesh proxy configuration."; }
  finally { busy.value = false; }
}

function parseModelRoutingLines(value: string, separator: "," | undefined): Record<string, string | string[]> {
  const result: Record<string, string | string[]> = {};
  for (const line of value.split(/\r?\n/).map((item) => item.trim()).filter(Boolean)) {
    const [key, rawValue] = line.split(/=(.*)/s, 2);
    if (!key?.trim() || !rawValue?.trim()) throw new Error("Model routing entries must use name=value form.");
    const values = separator ? rawValue.split(separator).map((item) => item.trim()).filter(Boolean) : rawValue.trim();
    if (Array.isArray(values) && values.length === 0) throw new Error("Fallback entries need at least one target model.");
    result[key.trim()] = values;
  }
  return result;
}

async function saveModelRouting() {
  if (!config.value || !window.confirm("Save model aliases and fallback chains in the OpenMesh registry?")) return;
  busy.value = true; clearMessage();
  try {
    const modelAliases = parseModelRoutingLines(aliasesDraft.value, undefined) as Record<string, string>;
    const modelFallbacks = parseModelRoutingLines(fallbacksDraft.value, ",") as Record<string, string[]>;
    config.value = await updateBuiltInProxyManagementConfig({ modelAliases, modelFallbacks });
    syncModelRoutingDrafts(config.value);
    notice.value = "Model aliases and fallback chains updated.";
  } catch (cause) { error.value = cause instanceof Error ? cause.message : "Unable to update model routing."; }
  finally { busy.value = false; }
}

async function saveUpstream() {
  if (!config.value) return;
  busy.value = true; clearMessage();
  try {
    const upstream = validateDraft();
    if (!window.confirm(`${editing.value ? "Update" : "Add"} ${upstream.id} in the OpenMesh provider registry?`)) return;
    const next = config.value.upstreams.filter((item) => item.id !== editing.value?.id);
    next.push(upstream as BuiltInProxyManagementUpstream);
    config.value = await updateBuiltInProxyManagementConfig({ upstreams: next });
    notice.value = editing.value ? "Upstream updated." : "Upstream added.";
    forceCloseEditor();
  } catch (cause) { error.value = cause instanceof Error ? cause.message : "Unable to update OpenMesh proxy configuration."; }
  finally { busy.value = false; }
}

async function removeUpstream(upstream: BuiltInProxyManagementUpstream) { if (config.value && window.confirm(`Remove ${upstream.id} from the OpenMesh provider registry?`)) await applyUpstreams(config.value.upstreams.filter((item) => item.id !== upstream.id), "Upstream removed."); }
async function toggleUpstream(upstream: BuiltInProxyManagementUpstream) { if (config.value && window.confirm(`${upstream.enabled ? "Disable" : "Enable"} ${upstream.id}?`)) await applyUpstreams(config.value.upstreams.map((item) => item.id === upstream.id ? { ...item, enabled: !item.enabled } : item), upstream.enabled ? "Upstream disabled." : "Upstream enabled."); }
async function setRoutingStrategy(event: Event) {
  if (!config.value) return;
  const select = event.target as HTMLSelectElement;
  const routingStrategy = select.value as BuiltInProxyManagementConfig["routingStrategy"];
  if (routingStrategy === config.value.routingStrategy) return;
  if (!window.confirm(`Change routing strategy to ${routingStrategy}?`)) { select.value = config.value.routingStrategy; return; }
  busy.value = true; clearMessage();
  try { config.value = await updateBuiltInProxyManagementConfig({ routingStrategy }); notice.value = "Routing strategy updated."; }
  catch (cause) { error.value = cause instanceof Error ? cause.message : "Unable to update routing strategy."; }
  finally { busy.value = false; }
}

onMounted(() => void loadConfig());
</script>

<template>
  <div class="proxy-providers-page animate-fade-in">
    <header class="proxy-providers-page__head">
      <div><p class="text-caption uppercase tracking-[0.18em] text-muted">OpenMesh runtime</p><h1 class="settings__title">Provider configuration</h1><p class="proxy-providers-page__lede">Manage the upstream registry owned by the running OpenMesh proxy. Credentials are write-only in this view and never returned.</p></div>
      <div class="proxy-providers-page__head-actions"><span class="proxy-providers-page__count">{{ config?.upstreams.length ?? 0 }} upstream{{ config?.upstreams.length === 1 ? "" : "s" }}</span><button type="button" class="btn-secondary" :disabled="loading || busy" @click="loadConfig"><RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': loading }" /> Refresh</button><button type="button" class="btn-primary" :disabled="!canEdit" @click="openCreate"><Plus class="h-3.5 w-3.5" /> Add upstream</button></div>
    </header>
    <div v-if="error" class="proxy-providers-page__alert is-error" role="alert"><AlertCircle class="h-4 w-4" /> {{ error }}</div><div v-else-if="notice" class="proxy-providers-page__alert is-success" role="status"><Check class="h-4 w-4" /> {{ notice }}</div>
    <section v-if="loading" class="workbench-card proxy-providers-page__loading"><CircleDashed class="h-5 w-5 animate-spin" /> Loading OpenMesh configuration…</section>
    <section v-else-if="config" class="proxy-providers-page__layout">
      <aside class="workbench-card proxy-providers-page__sections"><div class="proxy-providers-page__section-heading"><span class="text-caption uppercase tracking-[0.16em] text-muted">Routing</span><ShieldCheck class="h-4 w-4 text-muted" /></div><label class="proxy-providers-page__routing-label" for="routing-strategy">Selection strategy</label><select id="routing-strategy" class="input-luxury w-full" :value="config.routingStrategy" :disabled="busy" @change="setRoutingStrategy"><option value="first-compatible">First compatible</option><option value="round-robin">Round-robin</option><option value="fill-first">Fill-first</option></select><dl class="proxy-providers-page__summary"><div><dt>Client keys</dt><dd>{{ config.apiKeyCount }}</dd></div><div><dt>Aliases</dt><dd>{{ config.modelAliasCount }}</dd></div><div><dt>Retries</dt><dd>{{ config.maxRetries }}</dd></div></dl><p>Changes apply to the in-process server immediately. Bind and timeout changes remain lifecycle settings.</p><div class="proxy-providers-page__model-routing"><label><span>Model aliases <small>alias=target</small></span><textarea v-model="aliasesDraft" class="input-luxury w-full" rows="3" placeholder="fast-model=gpt-4o-mini" /></label><label><span>Fallback chains <small>model=target,target</small></span><textarea v-model="fallbacksDraft" class="input-luxury w-full" rows="3" placeholder="primary-model=fallback-model" /></label><button type="button" class="btn-secondary" :disabled="busy" @click="saveModelRouting"><Save class="h-3.5 w-3.5" /> Save model routing</button></div></aside>
      <section class="workbench-card proxy-providers-page__records"><div class="proxy-providers-page__records-head"><div><span class="text-caption uppercase tracking-[0.16em] text-muted">Upstreams</span><h2>{{ config.upstreams.length }} configured record{{ config.upstreams.length === 1 ? "" : "s" }}</h2></div><button type="button" class="btn-secondary" :disabled="!canEdit" @click="openCreate"><Plus class="h-3.5 w-3.5" /> Add</button></div><div v-if="config.upstreams.length === 0" class="proxy-providers-page__empty"><ShieldCheck class="h-6 w-6" /><strong>No upstreams configured</strong><span>Add a provider endpoint and model to make the built-in proxy useful.</span><button type="button" class="btn-primary" :disabled="!canEdit" @click="openCreate">Add upstream</button></div><div v-else class="proxy-providers-page__records-list"><article v-for="upstream in config.upstreams" :key="upstream.id" class="proxy-providers-page__record"><div class="proxy-providers-page__record-main"><div class="proxy-providers-page__record-title"><strong>{{ upstream.id }}</strong><span class="chip" :class="upstream.enabled ? 'chip-success' : 'chip-muted'">{{ upstream.enabled ? "Enabled" : "Disabled" }}</span></div><div class="proxy-providers-page__record-meta"><span>{{ upstream.protocol }}</span><span>{{ upstream.oauthProvider ? `OAuth · ${upstream.oauthProvider}` : upstream.apiKeyConfigured ? "Key configured" : "No key" }}</span><span>{{ upstream.models.length }} model{{ upstream.models.length === 1 ? "" : "s" }}</span><span>Priority {{ upstream.priority }}</span><span v-if="upstream.accountId">Account {{ upstream.accountId }}</span></div><code>{{ upstream.baseUrl }}</code></div><div class="proxy-providers-page__record-actions"><button type="button" class="btn-ghost" :disabled="busy" @click="toggleUpstream(upstream)">{{ upstream.enabled ? "Disable" : "Enable" }}</button><button type="button" class="btn-secondary" :disabled="busy" @click="openEdit(upstream)">Edit</button><button type="button" class="btn-danger" :disabled="busy" @click="removeUpstream(upstream)"><Trash2 class="h-3.5 w-3.5" /> Remove</button></div></article></div></section>
    </section>
    <section v-else class="workbench-card proxy-providers-page__loading"><AlertCircle class="h-5 w-5" /> Start the OpenMesh built-in proxy to edit providers.</section>
    <div v-if="editorOpen" class="proxy-providers-page__backdrop" role="presentation" @click.self="closeEditor"><form class="workbench-card proxy-providers-page__dialog" @submit.prevent="saveUpstream"><div class="proxy-providers-page__dialog-head"><div><span class="text-caption uppercase tracking-[0.16em] text-muted">{{ editing ? "Edit upstream" : "New upstream" }}</span><h2>OpenMesh provider</h2></div><button type="button" class="btn-ghost" :disabled="busy" aria-label="Close" @click="closeEditor"><X class="h-4 w-4" /></button></div><p class="proxy-providers-page__dialog-copy">Leave the API key blank while editing to preserve the existing credential. OAuth tokens stay in the OS credential manager.</p><div class="proxy-providers-page__form-grid"><label><span>Upstream id</span><input v-model="draft.id" class="input-luxury w-full" maxlength="160" required /></label><label><span>Protocol</span><select v-model="draft.protocol" class="input-luxury w-full"><option value="open-ai-compatible">OpenAI compatible</option><option value="anthropic">Anthropic</option><option value="gemini">Gemini</option></select></label><label class="proxy-providers-page__form-wide"><span>Base URL</span><input v-model="draft.baseUrl" class="input-luxury w-full" type="url" placeholder="https://api.example.com/v1" required /></label><label><span>API key <small>(optional when editing)</small></span><input v-model="draft.apiKey" class="input-luxury w-full" type="password" autocomplete="new-password" placeholder="Never displayed after save" /></label><label><span>OAuth provider <small>(optional)</small></span><select v-model="draft.oauthProvider" class="input-luxury w-full"><option value="">Use API key</option><option value="codex">Codex</option><option value="claude">Claude</option><option value="gemini">Gemini</option><option value="grok">Grok</option><option value="qwen">Qwen</option><option value="iflow">iFlow</option><option value="antigravity">Antigravity</option><option value="kimi">Kimi</option></select></label><label><span>Account id</span><input v-model="draft.accountId" class="input-luxury w-full" maxlength="160" placeholder="Required for OAuth" /></label><label><span>Priority</span><input v-model="draft.priority" class="input-luxury w-full" inputmode="numeric" /></label><label class="proxy-providers-page__toggle-field"><span>Record status</span><span><input v-model="draft.enabled" type="checkbox" /> Enabled</span></label><label class="proxy-providers-page__form-wide"><span>Models <small>one model id per line</small></span><textarea v-model="draft.models" class="input-luxury w-full proxy-providers-page__models" rows="6" placeholder="gpt-4o-mini\nreasoning-model" required /></label></div><div class="proxy-providers-page__dialog-actions"><button type="button" class="btn-secondary" :disabled="busy" @click="closeEditor">Cancel</button><button type="submit" class="btn-primary" :disabled="busy || !config"><Save class="h-3.5 w-3.5" /> {{ busy ? "Saving…" : "Save upstream" }}</button></div></form></div>
  </div>
</template>

<style scoped>
.proxy-providers-page { display: flex; flex-direction: column; gap: 1rem; max-width: 1180px; }
.proxy-providers-page__head, .proxy-providers-page__records-head, .proxy-providers-page__dialog-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; }
.proxy-providers-page__lede { max-width: 760px; margin: 0.35rem 0 0; color: var(--muted-foreground); font-size: 0.8rem; line-height: 1.5; }
.proxy-providers-page__head-actions, .proxy-providers-page__record-actions { display: flex; align-items: center; justify-content: flex-end; flex-wrap: wrap; gap: 0.45rem; }
.proxy-providers-page__count { color: var(--muted-foreground); font-size: 0.72rem; }
.proxy-providers-page__alert { display: flex; align-items: center; gap: 0.45rem; padding: 0.7rem 0.8rem; border: 1px solid var(--border); border-radius: 0.7rem; font-size: 0.76rem; }
.proxy-providers-page__alert.is-error { color: var(--accent-red); } .proxy-providers-page__alert.is-success { color: var(--accent-green); }
.proxy-providers-page__loading { display: flex; align-items: center; gap: 0.55rem; min-height: 8rem; padding: 1rem; color: var(--muted-foreground); font-size: 0.8rem; }
.proxy-providers-page__layout { display: grid; grid-template-columns: 230px minmax(0, 1fr); gap: 1rem; align-items: start; }
.proxy-providers-page__sections, .proxy-providers-page__records { padding: 1rem; }
.proxy-providers-page__section-heading { display: flex; align-items: center; justify-content: space-between; padding-bottom: 0.7rem; border-bottom: 1px solid var(--border); }
.proxy-providers-page__routing-label { display: block; margin: 0.9rem 0 0.45rem; color: var(--muted-foreground); font-size: 0.7rem; font-weight: 600; }
.proxy-providers-page__sections > p { color: var(--muted-foreground); font-size: 0.68rem; line-height: 1.4; }
.proxy-providers-page__summary { display: grid; gap: 0.45rem; margin: 1rem 0; padding: 0.8rem 0; border-block: 1px solid var(--border); }
.proxy-providers-page__summary div { display: flex; justify-content: space-between; gap: 0.5rem; color: var(--muted-foreground); font-size: 0.72rem; }
.proxy-providers-page__summary dd { margin: 0; color: var(--foreground); font-weight: 600; }
.proxy-providers-page__model-routing { display: flex; flex-direction: column; gap: 0.65rem; margin-top: 1rem; padding-top: 1rem; border-top: 1px solid var(--border); }
.proxy-providers-page__model-routing label { display: flex; flex-direction: column; gap: 0.35rem; color: var(--muted-foreground); font-size: 0.68rem; font-weight: 600; }
.proxy-providers-page__model-routing small { color: var(--muted-foreground); font-size: 0.62rem; font-weight: 400; }
.proxy-providers-page__records-head h2 { margin: 0.2rem 0 0; font-size: 1rem; font-weight: 600; }
.proxy-providers-page__records-list { display: flex; flex-direction: column; gap: 0.65rem; margin-top: 1rem; }
.proxy-providers-page__record { display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding: 0.8rem; border: 1px solid var(--border); border-radius: 0.7rem; background: var(--surface-2); }
.proxy-providers-page__record-main { min-width: 0; } .proxy-providers-page__record-title { display: flex; align-items: center; flex-wrap: wrap; gap: 0.5rem; }
.proxy-providers-page__record-title strong { color: var(--foreground); font-size: 0.82rem; } .proxy-providers-page__record-meta { display: flex; flex-wrap: wrap; gap: 0.45rem 0.8rem; margin: 0.4rem 0; color: var(--muted-foreground); font-size: 0.68rem; }
.proxy-providers-page__record-main code { display: block; max-width: 560px; overflow: hidden; color: var(--muted-foreground); font-size: 0.66rem; text-overflow: ellipsis; white-space: nowrap; }
.proxy-providers-page__empty { display: flex; flex-direction: column; align-items: center; gap: 0.55rem; padding: 3rem 1rem 2rem; color: var(--muted-foreground); text-align: center; font-size: 0.76rem; }
.proxy-providers-page__empty strong { color: var(--foreground); font-size: 0.86rem; }
.proxy-providers-page__backdrop { position: fixed; inset: 0; z-index: 80; display: grid; place-items: center; overflow: auto; padding: 1rem; background: rgb(0 0 0 / 0.58); }
.proxy-providers-page__dialog { width: min(700px, 100%); max-height: min(780px, 92vh); overflow: auto; padding: 1.1rem; }
.proxy-providers-page__dialog h2 { margin: 0.2rem 0 0; font-size: 1.05rem; font-weight: 600; } .proxy-providers-page__dialog-copy { margin: 0.85rem 0 0; color: var(--muted-foreground); font-size: 0.72rem; line-height: 1.45; }
.proxy-providers-page__form-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0.8rem; margin-top: 1rem; } .proxy-providers-page__form-grid label { display: flex; flex-direction: column; gap: 0.4rem; color: var(--muted-foreground); font-size: 0.7rem; font-weight: 600; } .proxy-providers-page__form-grid small { color: var(--muted-foreground); font-size: 0.64rem; font-weight: 400; } .proxy-providers-page__form-wide { grid-column: 1 / -1; } .proxy-providers-page__toggle-field span:last-child { display: inline-flex; align-items: center; gap: 0.45rem; min-height: 2.25rem; color: var(--foreground); font-weight: 500; } .proxy-providers-page__models { resize: vertical; font: 0.72rem ui-monospace, SFMono-Regular, Menlo, monospace; }
.proxy-providers-page__dialog-actions { display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 1rem; }
@media (max-width: 820px) { .proxy-providers-page__head { flex-direction: column; } .proxy-providers-page__head-actions { justify-content: flex-start; } .proxy-providers-page__layout { grid-template-columns: 1fr; } .proxy-providers-page__record { align-items: flex-start; flex-direction: column; } .proxy-providers-page__record-actions { justify-content: flex-start; } }
@media (max-width: 520px) { .proxy-providers-page__form-grid { grid-template-columns: 1fr; } .proxy-providers-page__form-wide { grid-column: auto; } .proxy-providers-page__record-actions { width: 100%; } .proxy-providers-page__record-actions button { flex: 1 1 auto; justify-content: center; } .proxy-providers-page__dialog-actions { flex-direction: column-reverse; } .proxy-providers-page__dialog-actions button { width: 100%; justify-content: center; } }
</style>
