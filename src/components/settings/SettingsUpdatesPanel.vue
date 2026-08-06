<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { ArrowUpRight, CheckCircle2, Loader2, RefreshCw } from "lucide-vue-next";
import { getAppVersion } from "../../lib/updates/appVersion";
import {
  checkForUpdates,
  hasKnownUpdate,
  maybeBackgroundUpdateCheck,
  openExternalUrl,
  readPersistedUpdateCheck,
  type PersistedUpdateCheck,
  type UpdateCheckStatus,
} from "../../lib/updates/updateCheck";

const emit = defineEmits<{
  badge: [available: boolean];
}>();

const appVersion = getAppVersion();
const status = ref<UpdateCheckStatus>("idle");
const errorMessage = ref("");
const persisted = ref<PersistedUpdateCheck | null>(readPersistedUpdateCheck());

const updateAvailable = computed(() =>
  hasKnownUpdate(persisted.value, appVersion),
);

const publishedLabel = computed(() => {
  const iso = persisted.value?.publishedAt;
  if (!iso) return "";
  try {
    return new Date(iso).toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return "";
  }
});

function syncBadge() {
  emit("badge", updateAvailable.value);
}

async function runCheck() {
  status.value = "checking";
  errorMessage.value = "";
  const result = await checkForUpdates(appVersion);
  if (result.status === "failed") {
    status.value = "failed";
    errorMessage.value = result.error;
    syncBadge();
    return;
  }
  persisted.value = result.persisted;
  status.value = result.status;
  syncBadge();
}

function openRelease() {
  const url =
    persisted.value?.htmlUrl ||
    "https://github.com/KJ-AIML/openmesh-agent-workbench/releases/latest";
  openExternalUrl(url);
}

onMounted(async () => {
  if (updateAvailable.value) {
    status.value = "update_available";
  }
  syncBadge();
  const quiet = await maybeBackgroundUpdateCheck(appVersion);
  if (quiet) {
    persisted.value = quiet;
    if (hasKnownUpdate(quiet, appVersion)) {
      status.value = "update_available";
    } else if (status.value === "idle") {
      // Keep idle unless user already checked this session.
    }
    syncBadge();
  }
});
</script>

<template>
  <div class="space-y-4">
    <div>
      <p class="settings-updates__title">About &amp; updates</p>
      <p class="settings-updates__desc">
        Compare this build to the latest GitHub release. Updates open in your
        browser — OpenMesh does not auto-install.
      </p>
    </div>

    <div class="settings-updates__row">
      <div>
        <p class="text-caption text-muted mb-1">Current version</p>
        <p class="settings-updates__version">
          v{{ appVersion }}
          <span
            v-if="updateAvailable"
            class="settings-updates__badge"
            title="A newer release was found on the last check"
          >
            Update available
          </span>
        </p>
      </div>
      <button
        type="button"
        class="btn-secondary inline-flex items-center gap-1.5"
        :disabled="status === 'checking'"
        @click="runCheck"
      >
        <Loader2 v-if="status === 'checking'" class="h-3.5 w-3.5 animate-spin" />
        <RefreshCw v-else class="h-3.5 w-3.5" />
        {{ status === "checking" ? "Checking…" : "Check for updates" }}
      </button>
    </div>

    <div
      v-if="status === 'up_to_date'"
      class="settings-updates__state settings-updates__state--ok"
    >
      <CheckCircle2 class="h-4 w-4 flex-shrink-0" style="color: var(--accent-green)" />
      <p>
        You’re on the latest release
        <span v-if="persisted?.latestVersion">(v{{ persisted.latestVersion }})</span>.
      </p>
    </div>

    <div
      v-else-if="status === 'failed'"
      class="settings-updates__state settings-updates__state--err"
    >
      <p>{{ errorMessage || "Update check failed." }}</p>
    </div>

    <div
      v-else-if="status === 'update_available' && persisted"
      class="settings-updates__panel space-y-3"
    >
      <div>
        <p class="settings-updates__panel-title">
          {{ persisted.name || `v${persisted.latestVersion}` }}
        </p>
        <p class="text-caption text-muted mt-1">
          Latest: v{{ persisted.latestVersion }}
          <span v-if="publishedLabel"> · {{ publishedLabel }}</span>
        </p>
      </div>
      <p v-if="persisted.bodyExcerpt" class="settings-updates__notes">
        {{ persisted.bodyExcerpt }}
      </p>
      <p v-else class="text-caption text-muted">
        No release notes on GitHub for this tag.
      </p>
      <div class="flex flex-wrap gap-2">
        <button type="button" class="btn-primary inline-flex items-center gap-1.5" @click="openRelease">
          Open release
          <ArrowUpRight class="h-3.5 w-3.5" />
        </button>
        <button type="button" class="btn-secondary" @click="openRelease">
          View on GitHub
        </button>
      </div>
      <p class="text-caption text-muted">
        Preview builds are unsigned. On macOS, “damaged” usually means Gatekeeper —
        run <code class="text-[0.95em]">xattr -cr /Applications/OpenMesh.app</code> or
        right-click → Open. Windows may show SmartScreen.
      </p>
    </div>

    <p v-else-if="status === 'idle'" class="text-caption text-muted">
      Last check:
      {{
        persisted?.checkedAt
          ? new Date(persisted.checkedAt).toLocaleString()
          : "not yet"
      }}
    </p>
  </div>
</template>

<style scoped>
.settings-updates__title {
  margin: 0;
  font-size: 0.92rem;
  font-weight: 600;
  letter-spacing: -0.015em;
}

.settings-updates__desc {
  margin: 0.25rem 0 0;
  font-size: 0.78rem;
  color: var(--muted-foreground);
}

.settings-updates__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  flex-wrap: wrap;
}

.settings-updates__version {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 650;
  letter-spacing: -0.02em;
  font-variant-numeric: tabular-nums;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.settings-updates__badge {
  font-size: 0.68rem;
  font-weight: 600;
  letter-spacing: 0.01em;
  padding: 0.15rem 0.45rem;
  border-radius: 6px;
  background: color-mix(in srgb, var(--accent-amber) 18%, transparent);
  color: var(--accent-amber);
  border: 1px solid color-mix(in srgb, var(--accent-amber) 35%, transparent);
}

.settings-updates__state {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  font-size: 0.8125rem;
  padding: 0.75rem 0.85rem;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--surface-1);
}

.settings-updates__state--err {
  color: #f87171;
  border-color: color-mix(in srgb, #f87171 35%, var(--border));
}

.settings-updates__panel {
  padding: 0.85rem 0.95rem;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--surface-1);
}

.settings-updates__panel-title {
  margin: 0;
  font-size: 0.9rem;
  font-weight: 600;
}

.settings-updates__notes {
  margin: 0;
  font-size: 0.78rem;
  line-height: 1.45;
  color: var(--muted-foreground);
  white-space: pre-wrap;
}
</style>
