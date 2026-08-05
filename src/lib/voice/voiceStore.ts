import { computed, ref } from "vue";
import type { VoicePhase, VoiceTurnLog } from "./types";

export type VoiceListenMode = "always" | "ptt";

const enabled = ref(false);
const phase = ref<VoicePhase>("off");
const interim = ref("");
const lastHeard = ref("");
const lastReply = ref("");
const lastError = ref<string | null>(null);
const lastActions = ref<string[]>([]);
const turns = ref<VoiceTurnLog[]>([]);
const speakEnabled = ref(true);
/** Jarvis default: click mic on, talk freely, pause when done speaking. */
const listenMode = ref<VoiceListenMode>("always");
const bargeInEnabled = ref(true);

export function useVoiceStore() {
  const statusLabel = computed(() => {
    switch (phase.value) {
      case "off":
        return "Voice off";
      case "idle":
        return listenMode.value === "ptt"
          ? "Hold mic to talk"
          : "Listening for you — just speak";
      case "listening":
        return "Listening… finish speaking to send";
      case "thinking":
        return "Working…";
      case "speaking":
        return bargeInEnabled.value
          ? "Speaking… (Stop to interrupt)"
          : "Speaking…";
      case "error":
        return lastError.value || "Voice error";
      default:
        return "Voice";
    }
  });

  function pushTurn(turn: VoiceTurnLog) {
    turns.value = [turn, ...turns.value].slice(0, 12);
  }

  function resetSessionVisuals() {
    interim.value = "";
    lastHeard.value = "";
    lastReply.value = "";
    lastError.value = null;
    lastActions.value = [];
  }

  return {
    enabled,
    phase,
    interim,
    lastHeard,
    lastReply,
    lastError,
    lastActions,
    turns,
    speakEnabled,
    listenMode,
    bargeInEnabled,
    statusLabel,
    pushTurn,
    resetSessionVisuals,
  };
}
