<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "../../lib/useStore";
import { useVoiceStore } from "../../lib/voice/voiceStore";

type CatalogEntry = {
  id: string;
  name: string;
  sizeBytes: number;
  license: string;
  languages: string[];
};

/** OpenRouter STT slugs (transcription modality — not the chat LLM list). */
const STT_PRESETS = [
  {
    id: "openai/whisper-large-v3",
    label: "Whisper Large v3 (recommended)",
  },
  { id: "openai/whisper-1", label: "Whisper 1 (faster, older)" },
  {
    id: "openai/gpt-4o-mini-transcribe",
    label: "GPT-4o Mini Transcribe",
  },
  { id: "openai/gpt-4o-transcribe", label: "GPT-4o Transcribe" },
] as const;

const LANG_PRESETS = [
  { id: "", label: "Auto-detect" },
  { id: "en", label: "English" },
  { id: "th", label: "Thai" },
  { id: "ja", label: "Japanese" },
  { id: "zh", label: "Chinese" },
  { id: "ko", label: "Korean" },
] as const;

const catalog = ref<CatalogEntry[]>([]);
const status = ref("Loading…");
const { listenMode, bargeInEnabled, speakEnabled } = useVoiceStore();
const { settings, saveSettings } = useStore();

const sttModel = computed({
  get: () => settings.value.voice?.sttModel || "openai/whisper-large-v3",
  set: (v: string) => {
    void saveSettings({
      voice: {
        ...settings.value.voice,
        sttModel: v.trim() || "openai/whisper-large-v3",
        sttLanguage: settings.value.voice?.sttLanguage ?? "",
      },
    });
  },
});

const sttLanguage = computed({
  get: () => settings.value.voice?.sttLanguage ?? "",
  set: (v: string) => {
    void saveSettings({
      voice: {
        ...settings.value.voice,
        sttModel: settings.value.voice?.sttModel || "openai/whisper-large-v3",
        sttLanguage: v,
      },
    });
  },
});

const customModel = ref("");
watch(
  sttModel,
  (m) => {
    if (!STT_PRESETS.some((p) => p.id === m)) customModel.value = m;
  },
  { immediate: true },
);

function applyCustomModel() {
  const v = customModel.value.trim();
  if (v) sttModel.value = v;
}

onMounted(async () => {
  try {
    catalog.value = await invoke<CatalogEntry[]>("voice_model_catalog");
    status.value = await invoke<string>("voice_model_status");
  } catch (e) {
    status.value = e instanceof Error ? e.message : String(e);
  }
});

function fmtSize(n: number) {
  return `${Math.round(n / 1_000_000)} MB`;
}
</script>

<template>
  <section class="voice-settings">
    <h3>Voice</h3>
    <p class="voice-settings__status">{{ status }}</p>

    <div class="voice-settings__block">
      <p class="voice-settings__label">Speech-to-text model</p>
      <p class="voice-settings__hint">
        This is <strong>not</strong> your chat LLM. Transcription uses OpenRouter’s
        audio API (or OpenAI Whisper). Chat replies still use Settings → Provider → Default model.
      </p>
      <select v-model="sttModel" class="voice-settings__select">
        <option v-for="p in STT_PRESETS" :key="p.id" :value="p.id">
          {{ p.label }}
        </option>
        <option v-if="customModel && !STT_PRESETS.some((p) => p.id === sttModel)" :value="sttModel">
          Custom: {{ sttModel }}
        </option>
      </select>
      <div class="voice-settings__custom">
        <input
          v-model="customModel"
          type="text"
          placeholder="Custom OpenRouter slug, e.g. openai/whisper-large-v3"
          @keydown.enter.prevent="applyCustomModel"
        />
        <button type="button" class="btn-secondary" @click="applyCustomModel">
          Use
        </button>
      </div>
    </div>

    <label class="voice-settings__row">
      <span>Spoken language hint</span>
      <select v-model="sttLanguage" class="voice-settings__select voice-settings__select--sm">
        <option v-for="l in LANG_PRESETS" :key="l.id || 'auto'" :value="l.id">
          {{ l.label }}
        </option>
      </select>
    </label>
    <p class="voice-settings__hint">
      If you speak Thai (or mix languages), set Thai — it usually improves accuracy a lot.
    </p>

    <label class="voice-settings__row">
      <span>Listen mode</span>
      <select v-model="listenMode">
        <option value="always">Always listen (Jarvis — default)</option>
        <option value="ptt">Push-to-talk (hold mic)</option>
      </select>
    </label>

    <p class="voice-settings__hint">
      Voice turns always run in <strong>Act</strong> mode (not Ask), so navigate / note /
      canvas tools have a real tool budget. Hard writes still need your approval card.
    </p>

    <label class="voice-settings__row">
      <span>Speak replies</span>
      <input v-model="speakEnabled" type="checkbox" />
    </label>

    <label class="voice-settings__row">
      <span>Barge-in (interrupt speech)</span>
      <input v-model="bargeInEnabled" type="checkbox" />
    </label>

    <h4>Local STT catalog</h4>
    <p class="voice-settings__hint">
      Offline models are listed only — downloads stay opt-in. Cloud STT is what runs today.
    </p>
    <ul class="voice-settings__catalog">
      <li v-for="m in catalog" :key="m.id">
        <strong>{{ m.name }}</strong>
        <span>{{ fmtSize(m.sizeBytes) }} · {{ m.license }} · {{ m.languages.join(", ") }}</span>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.voice-settings {
  display: grid;
  gap: 0.75rem;
}
.voice-settings h3 {
  margin: 0;
}
.voice-settings h4 {
  margin: 0.5rem 0 0;
  font-size: 0.9rem;
}
.voice-settings__status,
.voice-settings__hint {
  margin: 0;
  font-size: 0.82rem;
  opacity: 0.75;
}
.voice-settings__label {
  margin: 0 0 0.25rem;
  font-weight: 600;
  font-size: 0.9rem;
}
.voice-settings__block {
  display: grid;
  gap: 0.45rem;
}
.voice-settings__row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
  font-size: 0.88rem;
}
.voice-settings__select {
  width: 100%;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--foreground);
  padding: 0.45rem 0.6rem;
}
.voice-settings__select--sm {
  width: auto;
  min-width: 140px;
}
.voice-settings__custom {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0.45rem;
}
.voice-settings__custom input {
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--foreground);
  padding: 0.45rem 0.6rem;
  font-size: 0.82rem;
}
.voice-settings__catalog {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 0.45rem;
}
.voice-settings__catalog li {
  display: grid;
  gap: 0.15rem;
  padding: 0.55rem 0.65rem;
  border: 1px solid var(--border);
  border-radius: 8px;
  font-size: 0.8rem;
}
.voice-settings__catalog span {
  opacity: 0.65;
}
</style>
