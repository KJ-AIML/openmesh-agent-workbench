/** Turn agent/tool dumps into something safe for the Voice HUD + TTS. */

const MAX_HUD = 280;
const MAX_SPEAK = 220;

/**
 * Strip markdown dumps, JSON blobs, and tool-card noise into a short spoken line.
 */
export function formatVoiceReply(raw: string): string {
  let text = (raw || "").trim();
  if (!text) return "I didn't catch a reply.";

  // Local slash-tool cards: "### Sprint status ✓\n..."
  if (/^###\s+/m.test(text) || /^\s*\{/.test(text) || /"sprint"\s*:/.test(text)) {
    if (/sprint status/i.test(text) || /"sprint"\s*:/.test(text)) {
      return "I pulled sprint data, but voice stays short — open Sprint in the app for the board.";
    }
    if (/notes/i.test(text)) {
      return "I looked at your notes. Open Notes in the app for the list.";
    }
    if (/continuity/i.test(text)) {
      return "Continuity summary is ready in the Continuity page.";
    }
    return "I ran a workspace tool. Check Agent Chat for the full result.";
  }

  // Drop fenced code / json blocks
  text = text.replace(/```[\s\S]*?```/g, " ");
  // Drop obvious JSON objects/arrays
  text = text.replace(/\{[\s\S]{40,}\}/g, " ");
  text = text.replace(/\[[\s\S]{40,}\]/g, " ");
  // Collapse markdown headings / bullets
  text = text
    .replace(/^#{1,6}\s+/gm, "")
    .replace(/^\s*[-*]\s+/gm, "")
    .replace(/\*\*/g, "")
    .replace(/\s+/g, " ")
    .trim();

  if (!text || text.length < 2) {
    return "Done — check the app for details.";
  }

  // Prefer first 1–3 sentences
  const sentences = text.match(/[^.!?]+[.!?]+|[^.!?]+$/g) ?? [text];
  text = sentences.slice(0, 3).join(" ").trim();

  if (text.length > MAX_HUD) {
    text = `${text.slice(0, MAX_HUD - 1).trim()}…`;
  }
  return text;
}

export function formatVoiceSpeech(raw: string): string {
  const spoken = formatVoiceReply(raw);
  if (spoken.length <= MAX_SPEAK) return spoken;
  return `${spoken.slice(0, MAX_SPEAK - 1).trim()}…`;
}
