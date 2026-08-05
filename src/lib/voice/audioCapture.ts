/** Mic capture via MediaRecorder — avoids Web Speech (crashes Tauri/macOS TCC). */

export type CaptureHandle = {
  stop: () => void;
};

/**
 * Capture one spoken utterance: wait for voice, then stop after silence or max duration.
 */
export async function captureUtterance(opts?: {
  maxMs?: number;
  silenceMs?: number;
  onInterim?: (label: string) => void;
}): Promise<{ handle: CaptureHandle; done: Promise<Blob> }> {
  const maxMs = opts?.maxMs ?? 12_000;
  const silenceMs = opts?.silenceMs ?? 1_400;

  const stream = await navigator.mediaDevices.getUserMedia({
    audio: {
      echoCancellation: true,
      noiseSuppression: true,
      channelCount: 1,
    },
  });

  const mime = pickMimeType();
  const recorder = mime
    ? new MediaRecorder(stream, { mimeType: mime })
    : new MediaRecorder(stream);
  const chunks: BlobPart[] = [];
  recorder.ondataavailable = (ev) => {
    if (ev.data.size > 0) chunks.push(ev.data);
  };

  const audioCtx = new AudioContext();
  const source = audioCtx.createMediaStreamSource(stream);
  const analyser = audioCtx.createAnalyser();
  analyser.fftSize = 2048;
  source.connect(analyser);
  const data = new Uint8Array(analyser.fftSize);

  let stopRequested = false;
  const handle: CaptureHandle = {
    stop: () => {
      stopRequested = true;
      if (recorder.state === "recording") recorder.stop();
    },
  };

  const done = new Promise<Blob>((resolve, reject) => {
    let settled = false;
    const finish = (blob: Blob | null, err?: Error) => {
      if (settled) return;
      settled = true;
      cleanup();
      if (err) reject(err);
      else if (blob && blob.size > 0) resolve(blob);
      else reject(new Error("No audio captured."));
    };

    const cleanup = () => {
      try {
        stream.getTracks().forEach((t) => t.stop());
      } catch {
        /* ignore */
      }
      void audioCtx.close().catch(() => undefined);
    };

    recorder.onerror = () => finish(null, new Error("MediaRecorder failed."));
    recorder.onstop = () => {
      const type = recorder.mimeType || mime || "audio/webm";
      finish(new Blob(chunks, { type }));
    };

    recorder.start(200);
    opts?.onInterim?.("Listening…");

    const started = performance.now();
    let heardSpeech = false;
    let silenceStarted: number | null = null;

    const tick = () => {
      if (settled || stopRequested) return;
      analyser.getByteTimeDomainData(data);
      let sum = 0;
      for (let i = 0; i < data.length; i++) {
        const v = (data[i]! - 128) / 128;
        sum += v * v;
      }
      const rms = Math.sqrt(sum / data.length);
      const speaking = rms > 0.025;
      const elapsed = performance.now() - started;

      if (speaking) {
        heardSpeech = true;
        silenceStarted = null;
        opts?.onInterim?.("Listening…");
      } else if (heardSpeech) {
        if (silenceStarted == null) silenceStarted = performance.now();
        else if (performance.now() - silenceStarted >= silenceMs) {
          handle.stop();
          return;
        }
      }

      if (elapsed >= maxMs) {
        handle.stop();
        return;
      }
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);

    // Safety: if user never speaks, stop at max
    window.setTimeout(() => {
      if (!settled && recorder.state === "recording") handle.stop();
    }, maxMs + 200);
  });

  return { handle, done };
}

function pickMimeType(): string | undefined {
  const candidates = [
    "audio/webm;codecs=opus",
    "audio/webm",
    "audio/mp4",
    "audio/ogg;codecs=opus",
  ];
  for (const c of candidates) {
    if (typeof MediaRecorder !== "undefined" && MediaRecorder.isTypeSupported(c)) {
      return c;
    }
  }
  return undefined;
}

export async function ensureMicrophoneAccess(): Promise<void> {
  if (!navigator.mediaDevices?.getUserMedia) {
    throw new Error("Microphone API unavailable in this runtime.");
  }
  const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
  stream.getTracks().forEach((t) => t.stop());
}

export async function blobToBase64(blob: Blob): Promise<string> {
  const buf = await blob.arrayBuffer();
  const bytes = new Uint8Array(buf);
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
  }
  return btoa(binary);
}
