// DOMPurify needs a spec-accurate DOM (feature-detects tag/attribute support);
// happy-dom's HTML coverage is incomplete enough to make it under-sanitize,
// so this file runs under jsdom instead of the project-wide happy-dom env.
// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import {
  __clearMarkdownCaches,
  __markdownCacheSizes,
  renderMarkdownToSafeHtml,
  segmentChatContent,
} from "../src/lib/agentChat/markdown";

beforeEach(() => {
  __clearMarkdownCaches();
});

describe("segmentChatContent", () => {
  it("returns a single markdown segment for plain text", () => {
    const segs = segmentChatContent("hello **world**");
    expect(segs).toEqual([{ type: "markdown", content: "hello **world**" }]);
  });

  it("extracts a mermaid fence from surrounding markdown", () => {
    const text = "Before.\n\n```mermaid\ngraph TD;\nA-->B;\n```\n\nAfter.";
    const segs = segmentChatContent(text);
    expect(segs.map((s) => s.type)).toEqual(["markdown", "mermaid", "markdown"]);
    expect(segs[1]).toMatchObject({ type: "mermaid", content: "graph TD;\nA-->B;" });
    expect((segs[0] as { content: string }).content).toContain("Before.");
    expect((segs[2] as { content: string }).content).toContain("After.");
  });

  it("extracts canvas and artifact fences with their language tag", () => {
    const text =
      "```canvas\n{\"a\":1}\n```\n```artifact\nplain text block\n```";
    const segs = segmentChatContent(text);
    expect(segs).toEqual([
      { type: "artifact", lang: "canvas", content: '{"a":1}' },
      { type: "artifact", lang: "artifact", content: "plain text block" },
    ]);
  });

  it("leaves ordinary code fences (e.g. js) inside the markdown segment", () => {
    const text = "```js\nconst x = 1;\n```";
    const segs = segmentChatContent(text);
    expect(segs).toEqual([{ type: "markdown", content: text }]);
  });

  it("does not emit empty markdown segments between two special fences", () => {
    const text = "```mermaid\nA-->B\n```\n```mermaid\nC-->D\n```";
    const segs = segmentChatContent(text);
    expect(segs.map((s) => s.type)).toEqual(["mermaid", "mermaid"]);
  });
});

describe("renderMarkdownToSafeHtml", () => {
  it("renders headings, lists and emphasis", () => {
    const html = renderMarkdownToSafeHtml("# Title\n\n- one\n- two\n\n**bold** and *em*");
    expect(html).toContain("<h1>Title</h1>");
    expect(html).toContain("<li>one</li>");
    expect(html).toContain("<strong>bold</strong>");
    expect(html).toContain("<em>em</em>");
  });

  it("renders GFM tables", () => {
    const html = renderMarkdownToSafeHtml("| A | B |\n| - | - |\n| 1 | 2 |");
    expect(html).toContain("<table>");
    expect(html).toContain("<td>1</td>");
  });

  it("renders fenced code blocks with a language class", () => {
    const html = renderMarkdownToSafeHtml("```ts\nconst x = 1;\n```");
    expect(html).toContain('class="language-ts"');
    expect(html).toContain("const x = 1;");
  });

  it("strips script tags and inline event handlers (XSS safety)", () => {
    const html = renderMarkdownToSafeHtml(
      '<script>alert(1)</script>\n\n<img src="x" onerror="alert(1)">',
    );
    expect(html).not.toContain("<script>");
    expect(html).not.toContain("onerror");
  });

  it("strips javascript: URLs from links", () => {
    const html = renderMarkdownToSafeHtml("[click me](javascript:alert(1))");
    expect(html).not.toContain("javascript:");
  });
});

describe("markdown caches", () => {
  it("reuses segment and html results for identical source", () => {
    const text = "hello **world**\n\n```mermaid\nA-->B\n```";
    const a = segmentChatContent(text);
    const b = segmentChatContent(text);
    expect(a).toBe(b);

    const htmlA = renderMarkdownToSafeHtml("**bold**");
    const htmlB = renderMarkdownToSafeHtml("**bold**");
    expect(htmlA).toBe(htmlB);
    expect(htmlA).toContain("<strong>bold</strong>");

    expect(__markdownCacheSizes()).toEqual({ segments: 1, html: 1 });
  });

  it("clears caches via the test helper", () => {
    segmentChatContent("one");
    renderMarkdownToSafeHtml("two");
    expect(__markdownCacheSizes().segments).toBeGreaterThan(0);
    expect(__markdownCacheSizes().html).toBeGreaterThan(0);
    __clearMarkdownCaches();
    expect(__markdownCacheSizes()).toEqual({ segments: 0, html: 0 });
  });
});
