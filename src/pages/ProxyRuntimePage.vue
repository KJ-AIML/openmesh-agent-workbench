<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  AlertCircle,
  CheckCircle2,
  CircleDashed,
  RefreshCw,
  ShieldCheck,
  TriangleAlert,
} from "lucide-vue-next";
import {
  builtInProxyStatusLabel,
  getBuiltInProxyStatus,
  startBuiltInProxyFromSettings,
  stopBuiltInProxy,
  type BuiltInProxyStatus,
} from "../lib/builtinProxyClient";

const status = ref<BuiltInProxyStatus | null>(null);
const loading = ref(true);
const actionBusy = ref(false);
const error = ref("");

const statusTone = computed(() => {
  if (!status.value) return "is-loading";
  if (status.value.error) return "is-warning";
  return status.value.running ? "is-ready" : "is-neutral";
});

const runtimeLabel = computed(() =>
  status.value ? builtInProxyStatusLabel(status.value) : "Unavailable",
);

async function refresh() {
  loading.value = true;
  error.value = "";
  try {
    status.value = await getBuiltInProxyStatus();
  } catch {
    status.value = null;
    error.value = "Unable to load the OpenMesh built-in proxy status.";
  } finally {
    loading.value = false;
  }
}

async function toggleRuntime() {
  actionBusy.value = true;
  error.value = "";
  try {
    status.value = status.value?.running
      ? await stopBuiltInProxy()
      : await startBuiltInProxyFromSettings();
  } catch (cause) {
    error.value =
      cause instanceof Error ? cause.message : "Proxy lifecycle action failed.";
  } finally {
    actionBusy.value = false;
  }
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <div class="proxy-runtime-page animate-fade-in">
    <header class="proxy-runtime-page__head">
      <div>
        <p class="text-caption uppercase tracking-[0.18em] text-muted">
          OpenMesh runtime
        </p>
        <h1 class="settings__title">HTTP proxy</h1>
        <p class="proxy-runtime-page__lede">
          Optional local OpenAI-compatible listener. Agent Chat uses the
          configured provider directly and does not require this listener to
          be running.
        </p>
      </div>
      <div class="proxy-runtime-page__head-actions">
        <span
          class="proxy-runtime-page__status"
          :class="statusTone"
          data-testid="proxy-runtime-status"
        >
          <CheckCircle2 v-if="statusTone === 'is-ready'" class="h-4 w-4" />
          <TriangleAlert
            v-else-if="statusTone === 'is-warning'"
            class="h-4 w-4"
          />
          <CircleDashed v-else class="h-4 w-4" />
          {{ loading ? "Checking…" : runtimeLabel }}
        </span>
        <button
          type="button"
          class="btn-secondary"
          :disabled="loading || actionBusy"
          @click="refresh"
        >
          <RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': loading }" />
          Refresh
        </button>
      </div>
    </header>

    <div v-if="error" class="proxy-runtime-page__alert is-error" role="alert">
      <AlertCircle class="h-4 w-4" />
      {{ error }}
    </div>

    <section v-if="status" class="proxy-runtime-page__grid">
      <article class="workbench-card proxy-runtime-card proxy-runtime-card--wide">
        <div class="proxy-runtime-card__head">
          <div>
            <p class="text-caption uppercase tracking-[0.16em] text-muted">
              Lifecycle
            </p>
            <h2>OpenMesh-owned server</h2>
          </div>
          <ShieldCheck class="h-5 w-5 text-muted" />
        </div>
        <div class="proxy-runtime-facts">
          <div><span>Mode</span><strong>{{ status.mode }}</strong></div>
          <div><span>Ownership</span><strong>{{ status.ownership }}</strong></div>
          <div><span>Bind</span><strong>{{ status.bindHost || "—" }}</strong></div>
          <div><span>Port</span><strong>{{ status.port || "—" }}</strong></div>
          <div>
            <span>Client auth</span>
            <strong>{{ status.apiKeyConfigured ? "Configured" : "Missing" }}</strong>
          </div>
          <div><span>Upstreams</span><strong>{{ status.upstreamCount }}</strong></div>
          <div><span>Models</span><strong>{{ status.modelCount }}</strong></div>
        </div>
        <div class="proxy-runtime-endpoints">
          <div>
            <span>Local endpoint</span>
            <code>{{ status.endpoint || "Not listening" }}</code>
          </div>
        </div>
        <p v-if="status.error" class="proxy-runtime-card__error">{{ status.error }}</p>
        <div class="proxy-runtime-card__actions">
          <button
            type="button"
            class="btn-primary"
            :disabled="actionBusy"
            @click="toggleRuntime"
          >
            <CircleDashed v-if="actionBusy" class="h-3.5 w-3.5 animate-spin" />
            {{ status.running ? "Stop built-in proxy" : "Start from Provider settings" }}
          </button>
          <RouterLink to="/settings?section=provider" class="btn-secondary">
            Open Provider settings
          </RouterLink>
        </div>
      </article>

      <article class="workbench-card proxy-runtime-card">
        <div class="proxy-runtime-card__head">
          <div>
            <p class="text-caption uppercase tracking-[0.16em] text-muted">
              Built-in contract
            </p>
            <h2>Current slice</h2>
          </div>
          <span class="proxy-runtime-card__badge">Owned locally</span>
        </div>
        <dl class="proxy-runtime-config-list">
          <div><dt>OpenAI models</dt><dd>Available</dd></div>
          <div><dt>Chat completions</dt><dd>Available</dd></div>
          <div><dt>Responses API</dt><dd>Available</dd></div>
          <div><dt>SSE</dt><dd>Available</dd></div>
          <div><dt>Embeddings</dt><dd>Forwarded</dd></div>
          <div><dt>Claude/Gemini text translation</dt><dd>Available</dd></div>
        </dl>
        <p class="proxy-runtime-card__footnote">
          Native OAuth adapters, durable quota polling, and token-level
          provider streaming remain separate implementation slices with
          explicit contracts.
        </p>
      </article>
    </section>

    <section v-else-if="loading" class="workbench-card proxy-runtime-page__loading">
      <CircleDashed class="h-5 w-5 animate-spin" />
      Checking the OpenMesh built-in proxy…
    </section>

    <section v-else class="workbench-card proxy-runtime-page__loading">
      <AlertCircle class="h-5 w-5" />
      Runtime status is unavailable. Check Provider settings and try again.
    </section>
  </div>
</template>

<style scoped>
.proxy-runtime-page {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  max-width: 1180px;
}
.proxy-runtime-page__head,
.proxy-runtime-card__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}
.proxy-runtime-page__lede {
  max-width: 720px;
  margin: 0.35rem 0 0;
  color: var(--muted-foreground);
  font-size: 0.8rem;
  line-height: 1.5;
}
.proxy-runtime-page__head-actions {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  flex-wrap: wrap;
  justify-content: flex-end;
}
.proxy-runtime-page__status {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.45rem 0.65rem;
  border: 1px solid var(--border);
  border-radius: 999px;
  color: var(--muted-foreground);
  font-size: 0.72rem;
  font-weight: 600;
  white-space: nowrap;
}
.proxy-runtime-page__status.is-ready {
  color: var(--accent-green);
  border-color: color-mix(in oklab, var(--accent-green) 40%, var(--border));
}
.proxy-runtime-page__status.is-warning {
  color: var(--accent-amber);
  border-color: color-mix(in oklab, var(--accent-amber) 40%, var(--border));
}
.proxy-runtime-page__alert {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  padding: 0.7rem 0.8rem;
  border: 1px solid var(--border);
  border-radius: 0.7rem;
  color: var(--accent-red);
  font-size: 0.78rem;
}
.proxy-runtime-page__grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 1rem;
}
.proxy-runtime-card {
  min-width: 0;
  padding: 1rem;
}
.proxy-runtime-card--wide {
  grid-column: span 2;
}
.proxy-runtime-card h2 {
  margin: 0.25rem 0 0;
  font-size: 1.05rem;
}
.proxy-runtime-facts {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.75rem;
  margin-top: 1rem;
}
.proxy-runtime-facts > div,
.proxy-runtime-endpoints > div {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}
.proxy-runtime-facts span,
.proxy-runtime-endpoints span {
  color: var(--muted-foreground);
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}
.proxy-runtime-facts strong {
  overflow: hidden;
  color: var(--foreground);
  font-size: 0.78rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.proxy-runtime-endpoints {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 0.75rem;
  margin-top: 1rem;
}
.proxy-runtime-endpoints code {
  overflow: auto;
  color: var(--foreground);
  font-size: 0.72rem;
}
.proxy-runtime-card__actions {
  display: flex;
  gap: 0.6rem;
  flex-wrap: wrap;
  margin-top: 1rem;
}
.proxy-runtime-card__badge {
  padding: 0.3rem 0.5rem;
  border: 1px solid var(--border);
  border-radius: 999px;
  color: var(--muted-foreground);
  font-size: 0.67rem;
  white-space: nowrap;
}
.proxy-runtime-config-list {
  display: grid;
  gap: 0.6rem;
  margin: 1rem 0 0;
}
.proxy-runtime-config-list > div {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  padding-bottom: 0.55rem;
  border-bottom: 1px solid var(--border);
}
.proxy-runtime-config-list dt {
  color: var(--muted-foreground);
  font-size: 0.76rem;
}
.proxy-runtime-config-list dd {
  margin: 0;
  color: var(--foreground);
  font-size: 0.76rem;
  text-align: right;
}
.proxy-runtime-card__footnote,
.proxy-runtime-card__error {
  margin: 1rem 0 0;
  color: var(--muted-foreground);
  font-size: 0.74rem;
  line-height: 1.5;
}
.proxy-runtime-card__error {
  color: var(--accent-red);
}
.proxy-runtime-page__loading {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-height: 8rem;
  color: var(--muted-foreground);
}
@media (max-width: 900px) {
  .proxy-runtime-page__head,
  .proxy-runtime-card__head {
    flex-direction: column;
  }
  .proxy-runtime-page__head-actions {
    justify-content: flex-start;
  }
  .proxy-runtime-facts {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .proxy-runtime-card--wide {
    grid-column: span 1;
  }
  .proxy-runtime-page__grid {
    grid-template-columns: 1fr;
  }
}
</style>
