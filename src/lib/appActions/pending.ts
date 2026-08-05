import { ref } from "vue";
import type { ActionIntent } from "./types";

export type PendingAction = {
  id: string;
  intent: ActionIntent;
  label: string;
  policy: "soft" | "hard";
  at: number;
};

const pending = ref<PendingAction[]>([]);

export function usePendingActions() {
  return { pending };
}

export function enqueuePendingAction(
  intent: ActionIntent,
  label: string,
  policy: "soft" | "hard",
): string {
  const id = `pending-${Date.now()}-${Math.random().toString(16).slice(2, 6)}`;
  pending.value = [
    { id, intent, label, policy, at: Date.now() },
    ...pending.value,
  ].slice(0, 12);
  return id;
}

export function takePendingAction(id: string): PendingAction | null {
  const found = pending.value.find((p) => p.id === id) ?? null;
  if (found) {
    pending.value = pending.value.filter((p) => p.id !== id);
  }
  return found;
}

export function dismissPendingAction(id: string): void {
  pending.value = pending.value.filter((p) => p.id !== id);
}

export function clearPendingActions(): void {
  pending.value = [];
}
