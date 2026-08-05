/**
 * @deprecated Web Speech crashes Tauri on macOS (TCC SIGABRT for speech recognition).
 * Use `audioCapture.ts` (MediaRecorder) + `voice_transcribe` instead.
 */
export {
  ensureMicrophoneAccess,
} from "./audioCapture";

export function isSpeechRecognitionAvailable(): boolean {
  return false;
}
