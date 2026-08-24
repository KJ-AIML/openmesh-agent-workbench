<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { ChevronDown, CircleDot, AlertCircle } from "lucide-vue-next";
import {
  getListAccounts,
  getActiveAccounts,
  setActiveAccount,
  type AccountInfo,
} from "../lib/usageClient";

const emit = defineEmits<{
  "account-changed": [];
}>();

const accounts = ref<AccountInfo[]>([]);
const activeMap = ref<Record<string, string>>({});
const loading = ref(true);
const error = ref("");
const expandedProvider = ref<string | null>(null);
const busy = ref(false);

const groupedAccounts = computed(() => {
  const groups = new Map<string, AccountInfo[]>();
  for (const account of accounts.value) {
    const list = groups.get(account.provider) ?? [];
    list.push(account);
    groups.set(account.provider, list);
  }
  return groups;
});

const providers = computed(() => Array.from(groupedAccounts.value.keys()).sort());

async function load() {
  loading.value = true;
  error.value = "";
  try {
    const [accountList, active] = await Promise.all([
      getListAccounts(),
      getActiveAccounts(),
    ]);
    accounts.value = accountList;
    activeMap.value = active;
  } catch (requestError) {
    error.value = requestError instanceof Error ? requestError.message : "Account data unavailable.";
  } finally {
    loading.value = false;
  }
}

function toggleProvider(provider: string) {
  expandedProvider.value = expandedProvider.value === provider ? null : provider;
}

async function selectAccount(provider: string, accountName: string) {
  if (busy.value) return;
  busy.value = true;
  try {
    await setActiveAccount(provider, accountName);
    activeMap.value = { ...activeMap.value, [provider]: accountName };
    expandedProvider.value = null;
    emit("account-changed");
  } catch (requestError) {
    error.value = requestError instanceof Error ? requestError.message : "Unable to switch account.";
  } finally {
    busy.value = false;
  }
}

function activeName(provider: string): string {
  return activeMap.value[provider] ?? "none";
}

onMounted(() => {
  void load();
});
</script>

<template>
  <div class="account-switcher">
    <div v-if="loading" class="account-switcher__loading">
      Loading accounts…
    </div>
    <div v-else-if="error" class="account-switcher__error">
      <AlertCircle class="h-3 w-3" />
      <span>{{ error }}</span>
    </div>
    <template v-else>
      <div
        v-for="provider in providers"
        :key="provider"
        class="account-switcher__group"
      >
        <button
          type="button"
          class="account-switcher__trigger"
          @click="toggleProvider(provider)"
        >
          <span class="account-switcher__provider">{{ provider }}</span>
          <span class="account-switcher__active">{{ activeName(provider) }}</span>
          <ChevronDown
            class="h-3 w-3 account-switcher__chevron"
            :class="{ 'is-open': expandedProvider === provider }"
          />
        </button>
        <div v-if="expandedProvider === provider" class="account-switcher__dropdown">
          <button
            v-for="account in groupedAccounts.get(provider) ?? []"
            :key="account.name"
            type="button"
            class="account-switcher__option"
            :class="{
              'is-active': account.isActive,
              'is-disabled': !account.enabled,
              'is-unavailable': account.unavailable,
            }"
            :disabled="busy"
            @click="selectAccount(provider, account.name)"
          >
            <CircleDot
              v-if="account.isActive"
              class="h-3 w-3 account-switcher__dot is-active"
            />
            <span
              v-else-if="account.unavailable"
              class="account-switcher__dot-indicator is-unavailable"
            />
            <span
              v-else-if="!account.enabled"
              class="account-switcher__dot-indicator is-disabled"
            />
            <span v-else class="account-switcher__dot-indicator" />
            <span class="account-switcher__option-name">{{ account.name }}</span>
          </button>
          <div
            v-if="(groupedAccounts.get(provider) ?? []).length === 0"
            class="account-switcher__empty"
          >
            No accounts
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.account-switcher {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  padding: 0.25rem 0;
}

.account-switcher__loading,
.account-switcher__error {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.4rem 0.75rem;
  color: var(--muted-foreground);
  font-size: 0.66rem;
}

.account-switcher__error {
  color: var(--accent-red);
}

.account-switcher__group {
  position: relative;
}

.account-switcher__trigger {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  width: 100%;
  padding: 0.35rem 0.75rem;
  border-radius: 0.5rem;
  background: transparent;
  border: none;
  color: var(--muted-foreground);
  font-size: 0.68rem;
  cursor: pointer;
  transition: background 0.15s ease;
}

.account-switcher__trigger:hover {
  background: var(--surface-2);
}

.account-switcher__provider {
  font-weight: 600;
  color: var(--foreground);
  font-size: 0.66rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  min-width: 3.5rem;
}

.account-switcher__active {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--muted-foreground);
  font-size: 0.66rem;
}

.account-switcher__chevron {
  opacity: 0.5;
  transition: transform 0.15s ease;
}

.account-switcher__chevron.is-open {
  transform: rotate(180deg);
}

.account-switcher__dropdown {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
  padding: 0.2rem 0 0.2rem 0.75rem;
}

.account-switcher__option {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  width: 100%;
  padding: 0.3rem 0.5rem;
  border-radius: 0.4rem;
  background: transparent;
  border: none;
  color: var(--foreground);
  font-size: 0.66rem;
  cursor: pointer;
  transition: background 0.15s ease;
  text-align: left;
}

.account-switcher__option:hover {
  background: var(--surface-highlight);
}

.account-switcher__option.is-active {
  color: var(--foreground);
  font-weight: 500;
}

.account-switcher__option.is-disabled {
  color: var(--muted-foreground);
  opacity: 0.6;
}

.account-switcher__option.is-unavailable {
  color: var(--accent-red);
  opacity: 0.8;
}

.account-switcher__dot {
  color: var(--accent-green);
}

.account-switcher__dot-indicator {
  width: 0.5rem;
  height: 0.5rem;
  border-radius: 50%;
  background: var(--border-strong);
}

.account-switcher__dot-indicator.is-disabled {
  background: var(--muted-foreground);
  opacity: 0.4;
}

.account-switcher__dot-indicator.is-unavailable {
  background: var(--accent-red);
}

.account-switcher__option-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.account-switcher__empty {
  padding: 0.3rem 0.5rem;
  color: var(--muted-foreground);
  font-size: 0.64rem;
  opacity: 0.7;
}
</style>
