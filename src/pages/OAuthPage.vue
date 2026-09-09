<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  AlertCircle,
  CheckCircle2,
  CircleDashed,
  ExternalLink,
  KeyRound,
  RefreshCw,
  ShieldCheck,
  TriangleAlert,
} from "lucide-vue-next";
import {
  builtInProxyStatusLabel,
  getBuiltInProxyManagementConfig,
  getBuiltInProxyStatus,
  startBuiltInProxyFromSettings,
  stopBuiltInProxy,
  type BuiltInProxyManagementConfig,
  type BuiltInProxyManagementUpstream,
  type BuiltInProxyStatus,
} from "../lib/builtinProxyClient";
import {
  cancelOAuth,
  getOAuthStatus,
  OAUTH_MAX_POLL_MS,
  OAUTH_POLL_INTERVAL_MS,
  openOAuthUrl,
  OAUTH_PROVIDERS,
  startOAuth,
  type OAuthProviderId,
} from "../lib/oauthClient";

const status = ref<BuiltInProxyStatus | null>(null);
const config = ref<BuiltInProxyManagementConfig | null>(null);
const loading = ref(true);
const busy = ref(false);
const error = ref("");
const oauthBusy = ref<OAuthProviderId | null>(null);
const oauthMessage = ref("");

const statusTone = computed(() => {
  if (!status.value) return "is-loading";
  if (status.value.error) return "is-warning";
  return status.value.running ? "is-ready" : "is-neutral";
});

const runtimeLabel = computed(() =>
  status.value ? builtInProxyStatusLabel(status.value) : "Unavailable",
);

const providerAdapters = OAUTH_PROVIDERS;

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

async function login(provider: OAuthProviderId) {
  const descriptor = providerAdapters.find((item) => item.id === provider);
  if (!descriptor || (!descriptor.callbackSupported && !descriptor.deviceSupported)) return;
  oauthBusy.value = provider;
  oauthMessage.value = "Opening provider authorization…";
  error.value = "";
  let state: string | null = null;
  try {
    const started = await startOAuth(provider);
    state = started.state ?? null;
    if (!state) throw new Error("OpenMesh did not return an OAuth state.");
    if (started.flow === "device" && started.userCode) {
      oauthMessage.value = `Enter device code ${started.userCode} at ${started.verificationUri ?? started.url}.`;
    }
    await openOAuthUrl(started.url);
    const deadline = Date.now() + OAUTH_MAX_POLL_MS;
    while (Date.now() < deadline) {
      const result = await getOAuthStatus(state);
      if (result.status === "ok") {
        oauthMessage.value = `${descriptor.label} account connected.`;
        return;
      }
      if (result.status === "error") throw new Error(result.error ?? "Provider authorization failed.");
      oauthMessage.value = "Waiting for the provider callback…";
      await wait(OAUTH_POLL_INTERVAL_MS);
    }
    throw new Error("OAuth authorization timed out. You can try again.");
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : "OAuth authorization failed.";
    oauthMessage.value = "";
  } finally {
    if (state && error.value) {
      await cancelOAuth(state).catch(() => undefined);
    }
    oauthBusy.value = null;
  }
}

function protocolLabel(upstream: BuiltInProxyManagementUpstream): string {
  switch (upstream.protocol) {
    case "anthropic":
      return "Anthropic";
    case "gemini":
      return "Gemini";
    default:
      return "OpenAI-compatible";
  }
}

async function refresh() {
  loading.value = true;
  error.value = "";
  try {
    const nextStatus = await getBuiltInProxyStatus();
    status.value = nextStatus;
    try {
      config.value = await getBuiltInProxyManagementConfig();
    } catch {
      config.value = null;
    }
  } catch (cause) {
    status.value = null;
    config.value = null;
    error.value = cause instanceof Error ? cause.message : "Unable to load the OpenMesh proxy status.";
  } finally {
    loading.value = false;
  }
}

async function toggleRuntime() {
  busy.value = true;
  error.value = "";
  try {
    status.value = status.value?.running
      ? await stopBuiltInProxy()
      : await startBuiltInProxyFromSettings();
    if (status.value.running) config.value = await getBuiltInProxyManagementConfig();
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : "Proxy lifecycle action failed.";
  } finally {
    busy.value = false;
  }
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <div class="oauth-page animate-fade-in">
    <header class="oauth-page__head">
      <div>
        <p class="text-caption uppercase tracking-[0.18em] text-muted">OpenMesh runtime</p>
        <h1 class="settings__title">Connections</h1>
        <p class="oauth-page__lede">
          OAuth connections for provider accounts. Chat API keys live in Settings → Provider. The HTTP proxy listener is optional.
        </p>
      </div>
      <div class="oauth-page__head-actions">
        <span class="oauth-page__status" :class="statusTone" data-testid="oauth-runtime-status">
          <CheckCircle2 v-if="statusTone === 'is-ready'" class="h-4 w-4" />
          <TriangleAlert v-else-if="statusTone === 'is-warning'" class="h-4 w-4" />
          <CircleDashed v-else class="h-4 w-4" />
          {{ loading ? "Checking…" : runtimeLabel }}
        </span>
        <button type="button" class="btn-secondary" :disabled="loading || busy" @click="refresh">
          <RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': loading }" />
          Refresh
        </button>
      </div>
    </header>

    <div v-if="error" class="oauth-page__alert" role="alert">
      <AlertCircle class="h-4 w-4" />
      {{ error }}
    </div>

    <section v-if="status" class="oauth-page__grid">
      <article class="workbench-card oauth-card oauth-card--wide">
        <div class="oauth-card__head">
          <div>
            <p class="text-caption uppercase tracking-[0.16em] text-muted">Built-in data plane</p>
            <h2>OpenMesh-owned proxy</h2>
          </div>
          <ShieldCheck class="h-5 w-5 text-muted" />
        </div>
        <div class="oauth-facts">
          <div><span>Ownership</span><strong>{{ status.ownership }}</strong></div>
          <div><span>Mode</span><strong>{{ status.mode }}</strong></div>
          <div><span>Endpoint</span><code>{{ status.endpoint || "Not listening" }}</code></div>
          <div><span>Client auth</span><strong>{{ status.apiKeyConfigured ? "Configured" : "Missing" }}</strong></div>
          <div><span>Upstreams</span><strong>{{ status.upstreamCount }}</strong></div>
          <div><span>Models</span><strong>{{ status.modelCount }}</strong></div>
        </div>
        <p v-if="status.error" class="oauth-card__error">{{ status.error }}</p>
        <div class="oauth-card__actions">
          <button type="button" class="btn-primary" :disabled="busy" @click="toggleRuntime">
            <CircleDashed v-if="busy" class="h-3.5 w-3.5 animate-spin" />
            {{ status.running ? "Stop built-in proxy" : "Start built-in proxy" }}
          </button>
          <RouterLink to="/proxy-providers" class="btn-secondary">
            <KeyRound class="h-3.5 w-3.5" />
            Provider registry
          </RouterLink>
        </div>
      </article>

      <article class="workbench-card oauth-card">
        <div class="oauth-card__head">
          <div>
            <p class="text-caption uppercase tracking-[0.16em] text-muted">Native adapters</p>
            <h2>OAuth capability map</h2>
          </div>
          <span class="oauth-card__badge">Explicit status</span>
        </div>
        <ul class="oauth-adapter-list">
          <li v-for="provider in providerAdapters" :key="provider.id">
            <span><strong>{{ provider.label }}</strong><small>{{ provider.description }}</small></span>
            <button
              v-if="provider.callbackSupported || provider.deviceSupported"
              type="button"
              class="btn-secondary oauth-adapter-list__action"
              :disabled="busy || oauthBusy !== null"
              @click="login(provider.id)"
            >
              <CircleDashed v-if="oauthBusy === provider.id" class="h-3.5 w-3.5 animate-spin" />
              {{ oauthBusy === provider.id ? "Waiting…" : provider.deviceSupported ? "Device login" : "Connect" }}
            </button>
            <span v-else class="oauth-adapter-list__state">Endpoint required</span>
          </li>
        </ul>
        <p v-if="oauthMessage" class="oauth-card__hint">{{ oauthMessage }}</p>
      </article>

      <article v-if="config" class="workbench-card oauth-card oauth-card--wide">
        <div class="oauth-card__head">
          <div>
            <p class="text-caption uppercase tracking-[0.16em] text-muted">Configured upstreams</p>
            <h2>{{ config.upstreamCount }} provider records</h2>
          </div>
          <RouterLink to="/proxy-providers" class="oauth-card__link">
            Manage <ExternalLink class="h-3.5 w-3.5" />
          </RouterLink>
        </div>
        <div v-if="config.upstreams.length" class="oauth-upstream-list">
          <div v-for="upstream in config.upstreams" :key="upstream.id" class="oauth-upstream">
            <div>
              <strong>{{ upstream.id }}</strong>
              <span>{{ protocolLabel(upstream) }} · {{ upstream.models.length }} models</span>
            </div>
            <div class="oauth-upstream__meta">
              <span>{{ upstream.enabled ? "Enabled" : "Disabled" }}</span>
              <span>{{ upstream.apiKeyConfigured ? "Key configured" : "Key missing" }}</span>
            </div>
          </div>
        </div>
        <p v-else class="oauth-card__empty">No upstreams are configured yet. Add one in the provider registry.</p>
      </article>
    </section>

    <section v-else-if="loading" class="workbench-card oauth-page__loading">
      <CircleDashed class="h-5 w-5 animate-spin" />
      Checking the OpenMesh built-in proxy…
    </section>
  </div>
</template>

<style scoped>
.oauth-page { display: flex; flex-direction: column; gap: 1rem; max-width: 1180px; }
.oauth-page__head, .oauth-card__head { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; }
.oauth-page__lede { max-width: 720px; margin: 0.35rem 0 0; color: var(--muted-foreground); font-size: 0.8rem; line-height: 1.5; }
.oauth-page__head-actions, .oauth-card__actions, .oauth-card__link { display: flex; align-items: center; gap: 0.55rem; }
.oauth-page__head-actions { flex-wrap: wrap; justify-content: flex-end; }
.oauth-page__status { display: inline-flex; align-items: center; gap: 0.35rem; padding: 0.45rem 0.65rem; border: 1px solid var(--border); border-radius: 999px; color: var(--muted-foreground); font-size: 0.72rem; font-weight: 600; }
.oauth-page__status.is-ready { color: var(--accent-green); border-color: color-mix(in oklab, var(--accent-green) 40%, var(--border)); }
.oauth-page__status.is-warning { color: var(--accent-amber); border-color: color-mix(in oklab, var(--accent-amber) 40%, var(--border)); }
.oauth-page__alert { display: flex; align-items: center; gap: 0.45rem; padding: 0.7rem 0.8rem; border: 1px solid var(--border); border-radius: 0.7rem; color: var(--accent-red); font-size: 0.78rem; }
.oauth-page__grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 1rem; }
.oauth-card { min-width: 0; padding: 1rem; }
.oauth-card--wide { grid-column: 1 / -1; }
.oauth-card h2 { margin: 0.2rem 0 0; font-size: 1rem; }
.oauth-card__badge { border: 1px solid var(--border); border-radius: 999px; padding: 0.3rem 0.5rem; color: var(--muted-foreground); font-size: 0.68rem; }
.oauth-facts { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 0.75rem; margin-top: 1rem; }
.oauth-facts div { display: flex; flex-direction: column; gap: 0.22rem; min-width: 0; }
.oauth-facts span, .oauth-upstream span { color: var(--muted-foreground); font-size: 0.68rem; }
.oauth-facts strong, .oauth-facts code { overflow: hidden; color: var(--foreground); font-size: 0.78rem; text-overflow: ellipsis; white-space: nowrap; }
.oauth-card__error { margin: 0.9rem 0 0; color: var(--accent-red); font-size: 0.76rem; }
.oauth-card__actions { margin-top: 1rem; flex-wrap: wrap; }
.oauth-card__link { color: var(--accent-blue); font-size: 0.75rem; }
.oauth-adapter-list, .oauth-upstream-list { display: flex; flex-direction: column; gap: 0.55rem; margin: 1rem 0 0; padding: 0; list-style: none; }
.oauth-adapter-list li, .oauth-upstream { display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; padding: 0.65rem 0.7rem; border: 1px solid var(--border); border-radius: 0.65rem; }
.oauth-adapter-list li span:first-child, .oauth-upstream > div:first-child { display: flex; flex-direction: column; gap: 0.2rem; min-width: 0; }
.oauth-adapter-list small, .oauth-upstream span { color: var(--muted-foreground); font-size: 0.68rem; }
.oauth-adapter-list__state, .oauth-upstream__meta span { color: var(--accent-amber); font-size: 0.68rem; white-space: nowrap; }
.oauth-upstream__meta { display: flex; gap: 0.7rem; }
.oauth-card__empty, .oauth-page__loading { color: var(--muted-foreground); font-size: 0.78rem; }
.oauth-page__loading { display: flex; align-items: center; gap: 0.5rem; padding: 1rem; }
@media (max-width: 820px) { .oauth-page__head, .oauth-card__head { flex-direction: column; } .oauth-page__head-actions { justify-content: flex-start; } .oauth-page__grid { grid-template-columns: 1fr; } .oauth-facts { grid-template-columns: repeat(2, minmax(0, 1fr)); } .oauth-card--wide { grid-column: auto; } }
</style>
