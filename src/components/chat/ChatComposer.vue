<script setup lang="ts">
// Isolated composer so keystrokes don't re-render the message thread /
// session rail. Local input state stays here; parent only hears send/stop.
import { ref } from "vue";
import { Send, Square } from "lucide-vue-next";

const props = defineProps<{
  busy: boolean;
}>();

const emit = defineEmits<{
  send: [text: string];
  stop: [];
}>();

const input = ref("");
const textareaEl = ref<HTMLTextAreaElement | null>(null);

function submit() {
  const text = input.value.trim();
  if (!text || props.busy) return;
  emit("send", text);
  input.value = "";
}

function insertSlash(cmd: string) {
  input.value = cmd === "/tools" ? "/tools" : `${cmd} `;
}

function setText(text: string) {
  input.value = text;
}

function focus() {
  textareaEl.value?.focus();
}

defineExpose({ insertSlash, setText, focus });
</script>

<template>
  <footer class="chat-composer">
    <textarea
      ref="textareaEl"
      v-model="input"
      class="chat-composer__input"
      rows="2"
      placeholder="Ask the workspace…"
      @keydown.enter.exact.prevent="submit"
    />
    <button
      v-if="busy"
      type="button"
      class="btn-secondary chat-composer__send"
      @click="emit('stop')"
    >
      <Square :size="14" />
      Stop
    </button>
    <button
      v-else
      type="button"
      class="btn-primary chat-composer__send"
      :disabled="!input.trim()"
      @click="submit"
    >
      <Send :size="16" />
      Send
    </button>
  </footer>
</template>

<style scoped>
.chat-composer {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 0.65rem;
  align-items: end;
  flex-shrink: 0;
}

.chat-composer__input {
  resize: vertical;
  min-height: 56px;
  max-height: 160px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--foreground);
  padding: 0.75rem 0.9rem;
  font: inherit;
  font-size: 0.875rem;
  transition: border-color 0.15s ease, background 0.15s ease, box-shadow 0.15s ease;
}

.chat-composer__input:focus {
  outline: none;
  border-color: var(--border-strong);
  background: var(--surface-3);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent-blue) 18%, transparent);
}

.chat-composer__send {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  min-height: 40px;
}
</style>
