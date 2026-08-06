<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import {
  ArrowUpRight,
  CheckCircle2,
  Download,
  Loader2,
  RefreshCw,
} from "lucide-vue-next";
import { getAppVersion } from "../../lib/updates/appVersion";
import {
  downloadAndOpenInstaller,
  formatBytes,
  installButtonLabel,
  listenDownloadProgress,
  resolveInstallTarget,
  type DownloadProgress,
  type InstallUpdateStatus,
} from "../../lib/updates/installUpdate";
import { releaseHasInstallerAssets } from "../../lib/updates/releaseAssets";
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

const installStatus = ref<InstallUpdateStatus>("idle");
const installError = ref("");
const installNextSteps = ref("");
const progress = ref<DownloadProgress | null>(null);
const selectedAssetName = ref("");
const installersReady = ref(false);
const installUnsupported = ref(false);

let unlistenProgress: (() => void) | null = null;

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

const notesText = computed(() => {
  const excerpt = persisted.value?.bodyExcerpt?.trim() ?? "";
  if (excerpt) return excerpt;
  if (installersReady.value) return "Assets ready — see GitHub for full notes.";
  return "Release notes are not available yet.";
});

const progressLabel = computed(() => {
  const p = progress.value;
  if (!p) return "";
  if (p.percent != null) {
    const total =
      p.totalBytes != null ? ` · ${formatBytes(p.receivedBytes)} / ${formatBytes(p.totalBytes)}` : "";
    return `${p.percent}%${total}`;
  }
  if (p.receivedBytes > 0) return formatBytes(p.receivedBytes);
  return "Starting…";
});

const installBusy = computed(
  () =>
    installStatus.value === "resolving" ||
    installStatus.value === "downloading" ||
    installStatus.value === "opening",
);

const showInstallButton = computed(
  () => updateAvailable.value && !installUnsupported.value,
);

function syncBadge() {
  emit("badge", updateAvailable.value);
}

async function refreshInstallTarget() {
  const assets = persisted.value?.assets ?? [];
  installersReady.value = releaseHasInstallerAssets(assets);
  installUnsupported.value = false;
  selectedAssetName.value = "";

  if (!updateAvailable.value) return;

  const target = await resolveInstallTarget(assets);
  if (target.status === "unsupported") {
    installUnsupported.value = true;
    installStatus.value = "unsupported";
    return;
  }
  if (target.status === "assets_missing") {
    installStatus.value = "assets_missing";
    return;
  }
  selectedAssetName.value = target.asset.name;
  if (installStatus.value === "assets_missing" || installStatus.value === "unsupported") {
    installStatus.value = "idle";
  }
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
  await refreshInstallTarget();
}

function openRelease() {
  const url =
    persisted.value?.htmlUrl ||
    "https://github.com/KJ-AIML/openmesh-agent-workbench/releases/latest";
  openExternalUrl(url);
}

async function runInstall() {
  installError.value = "";
  installNextSteps.value = "";
  progress.value = null;
  installStatus.value = "resolving";

  const assets = persisted.value?.assets ?? [];
  const target = await resolveInstallTarget(assets);
  if (target.status === "unsupported") {
    installUnsupported.value = true;
    installStatus.value = "unsupported";
    installError.value = "This platform is not supported for in-app install.";
    return;
  }
  if (target.status === "assets_missing") {
    installStatus.value = "assets_missing";
    installError.value =
      "Installers not ready yet — try again shortly (CI may still be uploading).";
    return;
  }

  selectedAssetName.value = target.asset.name;
  installStatus.value = "downloading";

  try {
    unlistenProgress?.();
    unlistenProgress = await listenDownloadProgress((p) => {
      progress.value = p;
    });
    const result = await downloadAndOpenInstaller(target.asset);
    installStatus.value = "opening";
    installNextSteps.value = result.nextSteps;
    installStatus.value = "opened";
    if (!result.opened) {
      installError.value = "Downloaded, but the installer did not open automatically.";
    }
  } catch (err) {
    installStatus.value = "failed";
    installError.value =
      err instanceof Error ? err.message : String(err || "Install failed.");
  } finally {
    unlistenProgress?.();
    unlistenProgress = null;
  }
}

onMounted(async () => {
  if (updateAvailable.value) {
    status.value = "update_available";
  }
  syncBadge();

  // Older localStorage rows lack `assets` — refresh so Install can resolve.
  const needsAssetRefresh =
    updateAvailable.value && !Array.isArray(persisted.value?.assets);

  if (needsAssetRefresh) {
    await runCheck();
  } else {
    await refreshInstallTarget();
    const quiet = await maybeBackgroundUpdateCheck(appVersion);
    if (quiet) {
      persisted.value = quiet;
      if (hasKnownUpdate(quiet, appVersion)) {
        status.value = "update_available";
      }
      syncBadge();
      await refreshInstallTarget();
    }
  }
});

onUnmounted(() => {
  unlistenProgress?.();
  unlistenProgress = null;
});
</script>

<template>
  <div class="space-y-4">
    <div>
      <p class="settings-updates__title">About &amp; updates</p>
      <p class="settings-updates__desc">
        Compare this build to the latest GitHub release. When an installer is
        ready for this Mac or PC, you can download and open it from here —
        OpenMesh does not silently replace the running app.
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
        :disabled="status === 'checking' || installBusy"
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
      <p class="settings-updates__notes">
        {{ notesText }}
      </p>

      <p
        v-if="installStatus === 'assets_missing'"
        class="text-caption"
        style="color: var(--accent-amber)"
      >
        Installers not ready yet — try again shortly. You can still open the
        release page.
      </p>
      <p
        v-else-if="installUnsupported"
        class="text-caption text-muted"
      >
        In-app install isn’t available on this platform — use Open release.
      </p>
      <p
        v-else-if="selectedAssetName"
        class="text-caption text-muted"
      >
        Installer for this machine: <code class="text-[0.95em]">{{ selectedAssetName }}</code>
      </p>

      <div class="flex flex-wrap gap-2">
        <button
          v-if="showInstallButton"
          type="button"
          class="btn-primary inline-flex items-center gap-1.5"
          :disabled="installBusy || installStatus === 'assets_missing'"
          @click="runInstall"
        >
          <Loader2
            v-if="installBusy"
            class="h-3.5 w-3.5 animate-spin"
          />
          <Download v-else class="h-3.5 w-3.5" />
          {{ installButtonLabel(installStatus) }}
        </button>
        <button
          type="button"
          class="btn-secondary inline-flex items-center gap-1.5"
          :disabled="installBusy"
          @click="openRelease"
        >
          Open release
          <ArrowUpRight class="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          class="btn-secondary"
          :disabled="installBusy"
          @click="openRelease"
        >
          View on GitHub
        </button>
      </div>

      <div
        v-if="installStatus === 'downloading'"
        class="settings-updates__progress"
      >
        <div class="settings-updates__progress-bar">
          <div
            class="settings-updates__progress-fill"
            :style="{
              width:
                progress?.percent != null
                  ? `${Math.min(100, progress.percent)}%`
                  : '30%',
              opacity: progress?.percent != null ? 1 : 0.55,
            }"
          />
        </div>
        <p class="text-caption text-muted mt-1">
          Downloading{{ selectedAssetName ? ` ${selectedAssetName}` : "" }}
          <span v-if="progressLabel"> — {{ progressLabel }}</span>
        </p>
      </div>

      <div
        v-if="installStatus === 'failed'"
        class="settings-updates__state settings-updates__state--err"
      >
        <p>{{ installError || "Download or install failed." }}</p>
      </div>

      <div
        v-if="installStatus === 'opened' && installNextSteps"
        class="settings-updates__state settings-updates__state--ok"
      >
        <CheckCircle2 class="h-4 w-4 flex-shrink-0" style="color: var(--accent-green)" />
        <p class="settings-updates__notes" style="white-space: pre-wrap">
          {{ installNextSteps }}
        </p>
      </div>

      <p class="text-caption text-muted">
        Preview builds are unsigned. On macOS, “damaged” usually means Gatekeeper —
        run <code class="text-[0.95em]">xattr -cr /Applications/OpenMesh.app</code> or
        right-click → Open. Windows may show SmartScreen. This flow opens the
        installer; it does not replace the running app in place.
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

.settings-updates__progress-bar {
  height: 6px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--border) 80%, transparent);
  overflow: hidden;
}

.settings-updates__progress-fill {
  height: 100%;
  border-radius: inherit;
  background: var(--accent-amber, #d4a017);
  transition: width 0.2s ease;
}
</style>
