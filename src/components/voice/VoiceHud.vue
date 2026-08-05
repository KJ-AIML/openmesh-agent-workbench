<script setup lang="ts">
import { computed } from "vue";
import { Mic, MicOff, Volume2, X, Square } from "lucide-vue-next";
import { useRouter } from "vue-router";
import { useVoiceStore } from "../../lib/voice/voiceStore";
import {
  bargeInVoice,
  disableVoice,
  setVoiceListenMode,
} from "../../lib/voice/voiceSession";

const router = useRouter();
const {
  enabled,
  phase,
  interim,
  lastHeard,
  lastReply,
  lastError,
  lastActions,
  statusLabel,
  speakEnabled,
  listenMode,
} = useVoiceStore();

const tip = computed(() => {
  if (listenMode.value === "always") {
    if (phase.value === "listening") {
      return "Speak naturally — pause when you’re done and I’ll reply.";
    }
    if (phase.value === "idle") {
      return "Mic is on. Talk anytime. Click the mic again to turn off.";
    }
    if (phase.value === "thinking") {
      return "Got it — working on that…";
    }
    if (phase.value === "speaking") {
      return "I’ll keep listening after I finish speaking.";
    }
  }
  if (listenMode.value === "ptt") {
    if (phase.value === "idle" || phase.value === "off") {
      return "Hold the title-bar mic while you talk, then release.";
    }
    if (phase.value === "listening") {
      return "Listening… release the mic when you’re done.";
    }
  }
  return null;
});

function close() {
  bargeInVoice();
  disableVoice();
}

function stopSpeakingNow() {
  bargeInVoice();
}

function toggleMode() {
  const next = listenMode.value === "ptt" ? "always" : "ptt";
  setVoiceListenMode(next, router);
}

function toggleSpeak() {
  speakEnabled.value = !speakEnabled.value;
}
</script>

<template>
  <Transition name="voice-hud">
    <aside
      v-if="enabled"
      class="voice-hud"
      :data-phase="phase"
      role="status"
      aria-live="polite"
    >
      <header class="voice-hud__head">
        <span class="voice-hud__pulse" aria-hidden="true">
          <Mic v-if="phase === 'listening'" class="h-3.5 w-3.5" />
          <Volume2 v-else-if="phase === 'speaking'" class="h-3.5 w-3.5" />
          <MicOff v-else class="h-3.5 w-3.5" />
        </span>
        <div class="voice-hud__meta">
          <strong>OpenMesh Voice</strong>
          <span>{{ statusLabel }}</span>
        </div>
        <button
          v-if="phase === 'speaking' || phase === 'thinking'"
          type="button"
          class="voice-hud__stop"
          title="Stop / barge-in"
          @click.stop="stopSpeakingNow"
        >
          <Square class="h-3 w-3" />
        </button>
        <button
          type="button"
          class="voice-hud__close"
          title="Turn voice off"
          @click.stop="close"
        >
          <X class="h-3.5 w-3.5" />
        </button>
      </header>

      <p v-if="tip" class="voice-hud__tip">{{ tip }}</p>

      <p v-if="interim" class="voice-hud__interim">{{ interim }}</p>
      <p v-if="lastHeard" class="voice-hud__heard">
        <span>You</span>{{ lastHeard }}
      </p>

      <p v-if="lastReply" class="voice-hud__reply">
        <span>Agent</span>{{ lastReply }}
      </p>

      <p v-if="lastError" class="voice-hud__err">{{ lastError }}</p>

      <div v-if="lastActions.length" class="voice-hud__actions">
        <span v-for="label in lastActions" :key="label">{{ label }}</span>
      </div>

      <footer class="voice-hud__foot">
        <button type="button" class="voice-hud__mode" @click.stop="toggleMode">
          {{ listenMode === "always" ? "Always on" : "PTT" }}
        </button>
        <button
          type="button"
          class="voice-hud__mode"
          :aria-pressed="speakEnabled"
          @click.stop="toggleSpeak"
        >
          {{ speakEnabled ? "Voice reply on" : "Voice reply off" }}
        </button>
      </footer>
    </aside>
  </Transition>
</template>

<style scoped>
.voice-hud {
  position: fixed;
  right: 18px;
  bottom: 18px;
  z-index: 80;
  width: min(340px, calc(100vw - 32px));
  max-height: min(42vh, 360px);
  overflow: auto;
  padding: 12px 14px;
  border-radius: 14px;
  border: 1px solid var(--border);
  background: color-mix(in oklab, var(--sidebar) 92%, black);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.35);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.voice-hud__head {
  display: flex;
  align-items: center;
  gap: 10px;
  position: sticky;
  top: 0;
  background: inherit;
  z-index: 1;
}

.voice-hud__pulse {
  width: 28px;
  height: 28px;
  border-radius: 999px;
  display: grid;
  place-items: center;
  background: var(--surface-3);
  color: var(--foreground);
  flex-shrink: 0;
}

.voice-hud[data-phase="listening"] .voice-hud__pulse {
  background: color-mix(in srgb, #3d8bfd 35%, var(--surface-3));
  animation: voice-pulse 1.2s ease-in-out infinite;
}

@keyframes voice-pulse {
  0%,
  100% {
    transform: scale(1);
  }
  50% {
    transform: scale(1.08);
  }
}

.voice-hud__meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.voice-hud__meta strong {
  font-size: 0.85rem;
}
.voice-hud__meta span {
  font-size: 0.72rem;
  opacity: 0.7;
}

.voice-hud__close,
.voice-hud__stop {
  border: none;
  background: transparent;
  color: inherit;
  opacity: 0.75;
  cursor: pointer;
  padding: 4px;
  border-radius: 6px;
  flex-shrink: 0;
}
.voice-hud__stop {
  color: var(--danger, #e66);
  opacity: 1;
}

.voice-hud__tip {
  margin: 0;
  font-size: 0.72rem;
  line-height: 1.35;
  opacity: 0.72;
}

.voice-hud__interim,
.voice-hud__heard,
.voice-hud__reply {
  margin: 0;
  font-size: 0.82rem;
  line-height: 1.35;
  word-break: break-word;
}
.voice-hud__heard span,
.voice-hud__reply span {
  display: block;
  font-size: 0.68rem;
  text-transform: uppercase;
  opacity: 0.5;
  margin-bottom: 2px;
}
.voice-hud__err {
  margin: 0;
  color: var(--danger, #e66);
  font-size: 0.78rem;
}
.voice-hud__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
.voice-hud__actions span {
  font-size: 0.7rem;
  padding: 2px 6px;
  border-radius: 999px;
  background: var(--surface-3);
}
.voice-hud__foot {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 0.68rem;
  opacity: 0.65;
}
.voice-hud__mode {
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: inherit;
  border-radius: 999px;
  padding: 2px 8px;
  font-size: 0.68rem;
  cursor: pointer;
}

.voice-hud-enter-active,
.voice-hud-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}
.voice-hud-enter-from,
.voice-hud-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
