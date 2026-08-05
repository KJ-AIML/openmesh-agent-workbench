import type { Router } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { cancelAgentEngineTurn } from "../agentEngineClient";
import { setAppContext } from "../appActions/context";
import { runVoiceBridgeTurn } from "../agentChat/voiceBridge";
import { useStore } from "../useStore";
import {
  blobToBase64,
  captureUtterance,
  ensureMicrophoneAccess,
  type CaptureHandle,
} from "./audioCapture";
import { sanitizeVoiceError } from "./errors";
import { stopSpeaking, speakText } from "./tts";
import { useVoiceStore } from "./voiceStore";

/** Controllable voice session — always-listen or push-to-talk + barge-in. */
class VoiceSessionController {
  private listenHandle: CaptureHandle | null = null;
  private turnId: string | null = null;
  private loopActive = false;
  private busy = false;
  private generation = 0;
  private pttCapture: CaptureHandle | null = null;
  private pttDone: Promise<Blob> | null = null;

  async enable(router: Router): Promise<void> {
    const store = useVoiceStore();
    if (store.enabled.value) return;

    await ensureMicrophoneAccess();

    store.enabled.value = true;
    store.phase.value = "idle";
    store.lastError.value = null;
    this.generation += 1;

    if (store.listenMode.value === "always") {
      this.loopActive = true;
      void this.runListenLoop(router, this.generation);
    }
  }

  disable(): void {
    const store = useVoiceStore();
    // Flip UI off first so the close button never waits on IPC/TTS.
    store.enabled.value = false;
    store.phase.value = "off";
    store.interim.value = "";
    this.loopActive = false;
    this.busy = false;
    this.generation += 1;
    this.listenHandle?.stop();
    this.listenHandle = null;
    this.pttCapture?.stop();
    this.pttCapture = null;
    this.pttDone = null;
    if (this.turnId) {
      void cancelAgentEngineTurn(this.turnId);
      this.turnId = null;
    }
    stopSpeaking();
  }

  cancelTurn(): void {
    if (this.turnId) {
      void cancelAgentEngineTurn(this.turnId);
      this.turnId = null;
    }
    this.listenHandle?.stop();
    this.listenHandle = null;
    stopSpeaking();
    this.busy = false;
    const store = useVoiceStore();
    if (store.enabled.value) store.phase.value = "idle";
  }

  /** Interrupt TTS / in-flight turn (barge-in). */
  bargeIn(): void {
    const store = useVoiceStore();
    if (!store.bargeInEnabled.value) return;
    stopSpeaking();
    this.cancelTurn();
  }

  async toggle(router: Router): Promise<void> {
    const store = useVoiceStore();
    if (store.enabled.value) {
      this.disable();
      return;
    }
    await this.enable(router);
  }

  setListenMode(mode: "always" | "ptt", router: Router): void {
    const store = useVoiceStore();
    store.listenMode.value = mode;
    if (!store.enabled.value) return;
    this.loopActive = false;
    this.generation += 1;
    this.listenHandle?.stop();
    this.listenHandle = null;
    store.phase.value = "idle";
    if (mode === "always") {
      this.loopActive = true;
      void this.runListenLoop(router, this.generation);
    }
  }

  /** Push-to-talk: start capturing on press. */
  async pttStart(): Promise<void> {
    const store = useVoiceStore();
    if (!store.enabled.value || store.listenMode.value !== "ptt") return;
    if (this.busy || this.pttCapture) return;

    if (store.phase.value === "speaking") {
      this.bargeIn();
    }

    store.phase.value = "listening";
    store.interim.value = "Hold… speaking now";
    try {
      const session = await captureUtterance({
        // PTT: end on button release, not silence. Cap length so we don't hang.
        silenceMs: 120_000,
        maxMs: 30_000,
        onInterim: () => {
          store.interim.value = "Listening… release mic to send";
        },
      });
      this.pttCapture = session.handle;
      this.pttDone = session.done;
    } catch (e) {
      store.phase.value = "error";
      store.lastError.value = sanitizeVoiceError(e);
    }
  }

  /** Push-to-talk: stop capture on release and run the turn. */
  async pttEnd(router: Router): Promise<void> {
    const store = useVoiceStore();
    if (!this.pttCapture || !this.pttDone) {
      if (store.enabled.value) store.phase.value = "idle";
      return;
    }
    const done = this.pttDone;
    this.pttCapture.stop();
    this.pttCapture = null;
    this.pttDone = null;
    try {
      const blob = await done;
      if (!store.enabled.value) return;
      store.interim.value = "Transcribing…";
      const heard = (await this.transcribeBlob(blob)).trim();
      if (!heard) {
        store.phase.value = "idle";
        return;
      }
      await this.handleUtterance(router, heard);
    } catch (e) {
      const msg = sanitizeVoiceError(e);
      if (msg === "Cancelled." || /No audio|stopped/i.test(String(e))) {
        store.phase.value = "idle";
        return;
      }
      store.phase.value = "error";
      store.lastError.value = msg;
    }
  }

  private async runListenLoop(router: Router, gen: number): Promise<void> {
    const store = useVoiceStore();
    while (this.loopActive && store.enabled.value && gen === this.generation) {
      if (this.busy) {
        await sleep(200);
        continue;
      }
      // During speaking, poll for barge-in via overlapping capture is heavy;
      // user can click Stop or switch to PTT. Always-listen waits until idle.
      if (store.phase.value === "speaking" || store.phase.value === "thinking") {
        await sleep(200);
        continue;
      }
      try {
        store.phase.value = "listening";
        store.interim.value = "";
        const session = await captureUtterance({
          onInterim: (t) => {
            store.interim.value = t;
          },
        });
        this.listenHandle = session.handle;
        const blob = await session.done;
        this.listenHandle = null;
        if (!this.loopActive || !store.enabled.value || gen !== this.generation) break;

        store.interim.value = "Got it — transcribing…";
        store.phase.value = "thinking";
        const heard = (await this.transcribeBlob(blob)).trim();
        if (!heard) {
          store.phase.value = "idle";
          store.interim.value = "";
          await sleep(400);
          continue;
        }
        store.lastHeard.value = heard;
        store.interim.value = "";
        await this.handleUtterance(router, heard);
      } catch (e) {
        this.listenHandle = null;
        if (!this.loopActive || !store.enabled.value || gen !== this.generation) break;
        const msg = sanitizeVoiceError(e);
        if (
          msg.includes("No audio") ||
          msg === "Cancelled." ||
          /stopped/i.test(String(e))
        ) {
          store.phase.value = "idle";
          await sleep(400);
          continue;
        }
        store.phase.value = "error";
        store.lastError.value = msg;
        await sleep(2500);
        if (this.loopActive) store.phase.value = "idle";
      }
    }
  }

  private async transcribeBlob(blob: Blob): Promise<string> {
    const { settings } = useStore();
    const audioBase64 = await blobToBase64(blob);
    const voice = settings.value.voice;
    return invoke<string>("voice_transcribe", {
      request: {
        audioBase64,
        mime: blob.type || "audio/webm",
        providerName: settings.value.provider?.name,
        baseUrl: settings.value.provider?.apiBaseUrl,
        sttModel: voice?.sttModel || undefined,
        sttLanguage: voice?.sttLanguage || undefined,
      },
    });
  }

  private async handleUtterance(router: Router, heard: string): Promise<void> {
    const store = useVoiceStore();
    const { currentProject, settings } = useStore();
    const projectPath = currentProject.value?.folderPath;
    if (!projectPath) {
      store.phase.value = "error";
      store.lastError.value = "Open a project first.";
      return;
    }

    this.busy = true;
    store.lastHeard.value = heard;
    store.interim.value = "";
    store.phase.value = "thinking";
    store.lastError.value = null;
    setAppContext({
      route: router.currentRoute.value.path,
      projectPath,
    });

    const id = `voice-${Date.now()}`;
    this.turnId = id;

    try {
      const result = await runVoiceBridgeTurn(router, {
        projectPath,
        heard,
        settings: settings.value,
        turnId: id,
      });

      if (result.error && !result.reply) {
        store.phase.value = "error";
        store.lastError.value = result.error;
        store.pushTurn({
          id,
          heard,
          reply: "",
          actions: [],
          at: Date.now(),
          error: result.error,
        });
        return;
      }

      store.lastReply.value = result.reply;
      store.lastActions.value = result.actionLabels;
      store.pushTurn({
        id,
        heard,
        reply: result.reply,
        actions: result.actionLabels.map((label) => ({
          action: "ui_navigate" as const,
          route: "",
          label,
        })),
        at: Date.now(),
        error: result.error,
      });

      if (store.speakEnabled.value && result.reply) {
        store.phase.value = "speaking";
        try {
          await speakText(result.reply);
        } catch (speakErr) {
          // Keep the text reply; show why audio failed.
          store.lastError.value = `Voice reply failed: ${sanitizeVoiceError(speakErr)}`;
        }
      }
      store.phase.value = "idle";
    } catch (e) {
      const msg = sanitizeVoiceError(e);
      store.phase.value = "error";
      store.lastError.value = msg;
      store.pushTurn({
        id,
        heard,
        reply: "",
        actions: [],
        at: Date.now(),
        error: msg,
      });
    } finally {
      this.turnId = null;
      this.busy = false;
    }
  }
}

const controller = new VoiceSessionController();

export async function enableVoice(router: Router): Promise<void> {
  await controller.enable(router);
}

export function disableVoice(): void {
  controller.disable();
}

export function cancelVoiceTurn(): void {
  controller.cancelTurn();
}

export function bargeInVoice(): void {
  controller.bargeIn();
}

export async function toggleVoice(router: Router): Promise<void> {
  await controller.toggle(router);
}

export function setVoiceListenMode(
  mode: "always" | "ptt",
  router: Router,
): void {
  controller.setListenMode(mode, router);
}

export async function voicePttStart(): Promise<void> {
  await controller.pttStart();
}

export async function voicePttEnd(router: Router): Promise<void> {
  await controller.pttEnd(router);
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}
