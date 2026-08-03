// Rich chat content rendering: Markdown (sanitized), plus special fences for
// Mermaid diagrams and structured "canvas"/"artifact" blocks. Fenced blocks in
// those languages are pulled out *before* Markdown parsing so they never pass
// through marked/DOMPurify as HTML — they're rendered by dedicated Vue
// components instead (see components/chat/).

import { marked } from "marked";
import DOMPurify from "dompurify";

marked.setOptions({ gfm: true, breaks: true });

export type ChatContentSegment =
  | { type: "markdown"; content: string }
  | { type: "mermaid"; content: string }
  | { type: "artifact"; lang: "canvas" | "artifact"; content: string };

const SPECIAL_FENCE_RE = /```[ \t]*(mermaid|canvas|artifact)[ \t]*\r?\n([\s\S]*?)```/gi;

/**
 * Splits raw assistant/user text into alternating Markdown and
 * special-block segments, in document order.
 */
export function segmentChatContent(text: string): ChatContentSegment[] {
  const segments: ChatContentSegment[] = [];
  let cursor = 0;
  let match: RegExpExecArray | null;

  SPECIAL_FENCE_RE.lastIndex = 0;
  while ((match = SPECIAL_FENCE_RE.exec(text))) {
    const [full, rawLang, body] = match;
    if (match.index > cursor) {
      pushMarkdown(segments, text.slice(cursor, match.index));
    }
    const lang = rawLang.toLowerCase();
    segments.push(
      lang === "mermaid"
        ? { type: "mermaid", content: body.replace(/\s+$/, "") }
        : { type: "artifact", lang: lang as "canvas" | "artifact", content: body.replace(/\s+$/, "") },
    );
    cursor = match.index + full.length;
  }

  if (cursor < text.length) pushMarkdown(segments, text.slice(cursor));
  if (segments.length === 0) segments.push({ type: "markdown", content: text });

  return segments;
}

function pushMarkdown(segments: ChatContentSegment[], chunk: string): void {
  if (chunk.trim().length === 0) return;
  segments.push({ type: "markdown", content: chunk });
}

/** Markdown -> sanitized HTML. Safe to feed straight into v-html. */
export function renderMarkdownToSafeHtml(source: string): string {
  const raw = marked.parse(source, { async: false }) as string;
  return DOMPurify.sanitize(raw);
}
