/** Sanitize voice/STT/agent errors for the HUD (no raw keys / huge bodies). */

const MAX_LEN = 180;

const SENSITIVE =
  /\b(sk-[a-zA-Z0-9_-]{8,}|Bearer\s+\S+|api[_-]?key["']?\s*[:=]\s*["']?[^"'\s]+)/gi;

export function sanitizeVoiceError(raw: unknown): string {
  let msg = raw instanceof Error ? raw.message : String(raw ?? "Voice error");
  msg = msg.replace(SENSITIVE, "[redacted]");
  msg = msg.replace(/\s+/g, " ").trim();
  if (!msg) return "Voice error";

  // Collapse common provider noise into short UX copy.
  if (/No API key/i.test(msg)) {
    return "No API key for speech. Set one in Settings → Provider.";
  }
  if (/401|Unauthorized/i.test(msg)) {
    return "Speech auth failed. Check your API key in Settings.";
  }
  if (/network|fetch failed|ECONNREFUSED|timed out|timeout/i.test(msg)) {
    return "Network error talking to speech/agent service.";
  }
  if (/Open a project/i.test(msg)) {
    return "Open a project first.";
  }
  if (/cancelled|canceled|aborted/i.test(msg)) {
    return "Cancelled.";
  }

  if (msg.length > MAX_LEN) {
    return `${msg.slice(0, MAX_LEN - 1)}…`;
  }
  return msg;
}
