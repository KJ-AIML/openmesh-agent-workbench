import { onUnmounted, ref } from "vue";
import { getQuotaStatus, type ProviderQuota } from "./usageClient";

export function useQuotaPolling(intervalMs: number = 60000) {
  const quotas = ref<ProviderQuota[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  let timer: ReturnType<typeof setInterval> | null = null;

  async function refresh() {
    loading.value = true;
    error.value = null;
    try {
      quotas.value = await getQuotaStatus();
    } catch (requestError) {
      error.value = requestError instanceof Error ? requestError.message : "Quota data unavailable.";
    } finally {
      loading.value = false;
    }
  }

  function start() {
    if (timer !== null) return;
    void refresh();
    timer = setInterval(() => {
      void refresh();
    }, intervalMs);
  }

  function stop() {
    if (timer !== null) {
      clearInterval(timer);
      timer = null;
    }
  }

  start();

  onUnmounted(() => {
    stop();
  });

  return { quotas, loading, error, refresh };
}
