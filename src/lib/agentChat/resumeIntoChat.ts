/**
 * Build OpenMesh Agent Chat sessions from scanned foreign transcripts.
 *
 * OpenMesh owns the continuation: we copy into a new local chat session and
 * never mutate Cursor/Claude/Codex/OpenCode/Gemini/Grok session files on disk.
 */

import {
  createChatMessage,
  createChatSession,
  deriveTitleFromMessage,
  isAutoTitleWorthy,
  type ChatImportProvenance,
  type ChatMessage,
  type ChatRole,
  type ChatSession,
} from "./chatSessions";

export type ForeignTranscriptMessage = {
  role: string;
  text: string;
};

export type ForeignTranscript = {
  tool: string;
  path: string;
  title?: string | null;
  messages: ForeignTranscriptMessage[];
  truncated: boolean;
  truncationNote?: string | null;
  previewOnly: boolean;
};

export type ResumeSourceMeta = {
  source: string;
  id: string;
  path?: string;
  title?: string;
  /** Fallback when transcript IPC is unavailable (scan preview only). */
  summaryPreview?: string;
};

export type ResumeMode = "summarize" | "import";

const SUMMARIZE_HEAD = 3;
const SUMMARIZE_TAIL = 3;
const EXCERPT_CHARS = 420;

function provenanceNote(meta: ResumeSourceMeta, mode: ResumeMode): string {
  const tool = meta.source || "provider";
  const pathLine = meta.path ? `\nSource path (read-only): ${meta.path}` : "";
  const modeLine =
    mode === "summarize"
      ? "Mode: summarize & continue (concise local context seed)."
      : "Mode: import full & continue (copy of readable history into OpenMesh).";
  return [
    "This OpenMesh Agent Chat session continues a copy of a scanned agent session.",
    "The original provider session on disk was not modified.",
    "Continue here with your configured Agent Engine.",
    modeLine,
    `Imported from: ${tool} · id ${meta.id}${pathLine}`,
  ].join("\n");
}

function normalizeRole(role: string): ChatRole | null {
  const r = role.toLowerCase();
  if (r === "user" || r === "human") return "user";
  if (r === "assistant" || r === "model" || r === "gemini" || r === "grok") {
    return "assistant";
  }
  if (r === "system") return "system";
  return null;
}

function clip(text: string, limit: number): string {
  const cleaned = text.replace(/\s+/g, " ").trim();
  if (cleaned.length <= limit) return cleaned;
  return `${cleaned.slice(0, limit - 1).trimEnd()}…`;
}

function formatTurn(m: ForeignTranscriptMessage, index: number): string {
  const role = normalizeRole(m.role) ?? m.role;
  return `${index + 1}. [${role}] ${clip(m.text, EXCERPT_CHARS)}`;
}

/** Deterministic offline summary from transcript messages (+ optional preview). */
export function buildLocalSummary(
  transcript: ForeignTranscript | null,
  meta: ResumeSourceMeta,
): string {
  const title =
    transcript?.title?.trim() ||
    meta.title?.trim() ||
    "Untitled scanned session";
  const messages = transcript?.messages ?? [];
  const lines: string[] = [
    `Summary of scanned ${meta.source} session: ${title}`,
    "",
  ];

  if (messages.length === 0) {
    const preview = meta.summaryPreview?.trim();
    if (preview) {
      lines.push("Preview-only context (full turns were not available):");
      lines.push(clip(preview, EXCERPT_CHARS * 2));
    } else {
      lines.push(
        "No readable message turns were available from this session file. Title/metadata only.",
      );
    }
  } else {
    const head = messages.slice(0, SUMMARIZE_HEAD);
    const tailStart = Math.max(SUMMARIZE_HEAD, messages.length - SUMMARIZE_TAIL);
    const middleDropped = Math.max(0, tailStart - SUMMARIZE_HEAD);
    lines.push(`Turns available: ${messages.length}`);
    lines.push("");
    lines.push("First turns:");
    head.forEach((m, i) => lines.push(formatTurn(m, i)));
    if (middleDropped > 0) {
      lines.push("");
      lines.push(`… ${middleDropped} middle turn(s) omitted …`);
      lines.push("");
      lines.push("Last turns:");
      messages.slice(tailStart).forEach((m, i) => lines.push(formatTurn(m, tailStart + i)));
    } else if (messages.length > SUMMARIZE_HEAD) {
      lines.push("");
      lines.push("Later turns:");
      messages.slice(SUMMARIZE_HEAD).forEach((m, i) =>
        lines.push(formatTurn(m, SUMMARIZE_HEAD + i)),
      );
    }
  }

  if (transcript?.truncated && transcript.truncationNote) {
    lines.push("");
    lines.push(`Note: ${transcript.truncationNote}`);
  } else if (transcript?.previewOnly) {
    lines.push("");
    lines.push(
      "Note: this provider session exposed preview/metadata only; OpenMesh could not import a full turn list.",
    );
  }

  return lines.join("\n").trim();
}

function mapTranscriptMessages(
  transcript: ForeignTranscript,
): { messages: ChatMessage[]; skipped: number } {
  const messages: ChatMessage[] = [];
  let skipped = 0;
  for (const m of transcript.messages) {
    const role = normalizeRole(m.role);
    if (!role) {
      skipped += 1;
      continue;
    }
    const text = m.text?.trim();
    if (!text) {
      skipped += 1;
      continue;
    }
    messages.push(createChatMessage(role, text));
  }
  return { messages, skipped };
}

function sessionTitle(
  meta: ResumeSourceMeta,
  transcript: ForeignTranscript | null,
  seeded: ChatMessage[],
): { title: string; titleIsDefault: boolean } {
  const fromMeta = transcript?.title?.trim() || meta.title?.trim();
  if (fromMeta) {
    return {
      title: fromMeta.length > 42 ? `${fromMeta.slice(0, 41).trimEnd()}…` : fromMeta,
      titleIsDefault: false,
    };
  }
  const firstUser = seeded.find((m) => m.role === "user");
  if (firstUser && isAutoTitleWorthy(firstUser.text)) {
    return { title: deriveTitleFromMessage(firstUser.text), titleIsDefault: false };
  }
  return {
    title: `From ${meta.source}`,
    titleIsDefault: false,
  };
}

/**
 * Create a new OpenMesh chat session seeded for continue-in-chat.
 * Does not persist — caller should prepend + persistSessionsAsync.
 */
export function buildResumedChatSession(
  mode: ResumeMode,
  meta: ResumeSourceMeta,
  transcript: ForeignTranscript | null,
): ChatSession {
  const importedFrom: ChatImportProvenance = {
    source: meta.source,
    id: meta.id,
    path: meta.path,
  };

  const session = createChatSession();
  session.importedFrom = importedFrom;
  session.messages.push(createChatMessage("system", provenanceNote(meta, mode)));

  if (mode === "summarize") {
    const summary = buildLocalSummary(transcript, meta);
    session.messages.push(createChatMessage("system", summary));
    session.messages.push(
      createChatMessage(
        "assistant",
        "Context loaded from the scanned session copy. Ask a follow-up — OpenMesh will continue with your Agent Engine (the original provider thread stays untouched).",
      ),
    );
  } else {
    if (transcript && transcript.messages.length > 0) {
      const { messages, skipped } = mapTranscriptMessages(transcript);
      session.messages.push(...messages);
      if (transcript.truncated && transcript.truncationNote) {
        session.messages.push(
          createChatMessage("system", `Import note: ${transcript.truncationNote}`),
        );
      }
      if (skipped > 0) {
        session.messages.push(
          createChatMessage(
            "system",
            `Skipped ${skipped} non-user/assistant or empty turn(s) during import.`,
          ),
        );
      }
      session.messages.push(
        createChatMessage(
          "assistant",
          "Imported a copy of the scanned session into OpenMesh Chat. This is not the live provider thread — continue here with your Agent Engine.",
        ),
      );
    } else {
      // Fall back to summarize-style seed when full turns unavailable.
      const summary = buildLocalSummary(transcript, meta);
      session.messages.push(createChatMessage("system", summary));
      session.messages.push(
        createChatMessage(
          "assistant",
          "Full message history was not available for this session file, so OpenMesh loaded a summary copy instead. The original provider session was not modified.",
        ),
      );
    }
  }

  const { title, titleIsDefault } = sessionTitle(meta, transcript, session.messages);
  session.title = title;
  session.titleIsDefault = titleIsDefault;
  session.updatedAt = Date.now();
  return session;
}
