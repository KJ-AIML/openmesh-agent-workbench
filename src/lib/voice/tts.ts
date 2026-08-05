import { invoke } from "@tauri-apps/api/core";
import { isTauriRuntime } from "../adapters/environment";
import { formatVoiceSpeech } from "./replyFormat";

/** Prefer native TTS via Tauri; fall back to Web Speech Synthesis. */
export async function speakText(text: string): Promise<void> {
  const clean = formatVoiceSpeech(text);
  if (!clean) return;

  if (isTauriRuntime()) {
    try {
      // Blocks until OS TTS finishes (native_tts waits on the child).
      await invoke("voice_speak", { text: clean });
      return;
    } catch (err) {
      // Surface failure after web fallback also fails.
      try {
        await speakWeb(clean);
        return;
      } catch {
        throw err instanceof Error ? err : new Error(String(err));
      }
    }
  }

  await speakWeb(clean);
}

function speakWeb(text: string): Promise<void> {
  return new Promise((resolve, reject) => {
    if (typeof window === "undefined" || !window.speechSynthesis) {
      reject(new Error("Web Speech Synthesis unavailable"));
      return;
    }
    window.speechSynthesis.cancel();
    const utter = new SpeechSynthesisUtterance(text);
    utter.rate = 1.05;
    utter.onend = () => resolve();
    utter.onerror = () => reject(new Error("Web Speech Synthesis failed"));
    window.speechSynthesis.speak(utter);
  });
}

/** Never await a blocking kill on the UI path. */
export function stopSpeaking(): void {
  if (typeof window !== "undefined" && window.speechSynthesis) {
    window.speechSynthesis.cancel();
  }
  if (isTauriRuntime()) {
    void invoke("voice_speak_stop").catch(() => undefined);
  }
}
