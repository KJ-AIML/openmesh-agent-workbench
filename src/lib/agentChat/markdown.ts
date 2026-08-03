// Rich chat content rendering: Markdown (sanitized), plus special fences for
// Mermaid diagrams and structured "canvas"/"artifact" blocks. Fenced blocks in
// those languages are pulled out *before* Markdown parsing so they never pass
// through marked/DOMPurify as HTML — they're rendered by dedicated Vue
// components instead (see components/chat/).
//
// Segment + HTML results are cached by source string so re-renders of the
// message list (and identical chunks across turns) skip marked/DOMPurify.

import { marked } from "marked";
import DOMPurify from "dompurify";

marked.setOptions({ gfm: true, breaks: true });

export type ChatContentSegment =
  | { type: "markdown"; content: string }
  | { type: "mermaid"; content: string }
  | { type: "artifact"; lang: "canvas" | "artifact"; content: string };

const SPECIAL_FENCE_RE = /```[ \t]*(mermaid|canvas|artifact)[ \t]*\r?\n([\s\S]*?)```/gi;

const MAX_CACHE_ENTRIES = 200;
const segmentCache = new Map<string, ChatContentSegment[]>();
const htmlCache = new Map<string, string>();

function boundedGet<V>(map: Map<string, V>, key: string, compute: () => V): V {
  const hit = map.get(key);
  if (hit !== undefined) {
    // Refresh insertion order for a simple LRU eviction policy.
    map.delete(key);
    map.set(key, hit);
    return hit;
  }
  const value = compute();
  map.set(key, value);
  if (map.size > MAX_CACHE_ENTRIES) {
    const oldest = map.keys().next().value;
    if (oldest !== undefined) map.delete(oldest);
  }
  return value;
}

/**
 * Splits raw assistant/user text into alternating Markdown and
 * special-block segments, in document order.
 */
export function segmentChatContent(text: string): ChatContentSegment[] {
  return boundedGet(segmentCache, text, () => segmentChatContentUncached(text));
}

function segmentChatContentUncached(text: string): ChatContentSegment[] {
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
  return boundedGet(htmlCache, source, () => {
    const raw = marked.parse(source, { async: false }) as string;
    return DOMPurify.sanitize(raw);
  });
}

/** Test helper — clears segment/HTML caches between cases. */
export function __clearMarkdownCaches(): void {
  segmentCache.clear();
  htmlCache.clear();
}

/** Test helper — current cache sizes. */
export function __markdownCacheSizes(): { segments: number; html: number } {
  return { segments: segmentCache.size, html: htmlCache.size };
}
