import { describe, it, expect } from "vitest";
import {
  validateContextSource,
  validateContextDocument,
  isValidIsoTimestamp,
  isJsonSafe,
} from "@/domain/context/validators";
import type { ContextSource, ContextDocument } from "@/domain/context/types";

const VALID_SOURCE: ContextSource = {
  id: "abc123",
  schemaVersion: "1.0.0",
  kind: "doc",
  projectId: "proj-1",
  canonicalRef: "openmesh://project/proj-1/doc/foo.md",
  title: "Foo",
  sensitivity: "private",
  agentContextEnabled: false,
  createdAt: "2026-07-01T10:00:00.000Z",
  updatedAt: "2026-07-05T14:30:00.000Z",
};

const VALID_DOCUMENT: ContextDocument = {
  id: "doc-1",
  schemaVersion: "1.0.0",
  sourceId: "abc123",
  kind: "doc",
  projectId: "proj-1",
  canonicalRef: "openmesh://project/proj-1/doc/foo.md",
  title: "Foo",
  text: "Hello world",
  sensitivity: "private",
  agentContextEnabled: false,
  observedAt: "2026-07-05T03:00:00.000Z",
  freshness: {
    state: "fresh",
    observedAt: "2026-07-05T03:00:00.000Z",
  },
};

describe("validators — ContextSource", () => {
  it("accepts a valid source", () => {
    const result = validateContextSource(VALID_SOURCE);
    expect(result.valid).toBe(true);
    expect(result.errors).toHaveLength(0);
  });

  it("rejects null", () => {
    expect(validateContextSource(null).valid).toBe(false);
  });

  it("rejects missing required fields", () => {
    const result = validateContextSource({ ...VALID_SOURCE, id: "", projectId: "" });
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.path === "id")).toBe(true);
    expect(result.errors.some((e) => e.path === "projectId")).toBe(true);
  });

  it("rejects wrong schema version", () => {
    const result = validateContextSource({ ...VALID_SOURCE, schemaVersion: "0.9.0" });
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.path === "schemaVersion")).toBe(true);
  });

  it("rejects invalid kind", () => {
    const result = validateContextSource({ ...VALID_SOURCE, kind: "bogus" as never });
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.path === "kind")).toBe(true);
  });

  it("rejects invalid sensitivity", () => {
    const result = validateContextSource({ ...VALID_SOURCE, sensitivity: "top-secret" as never });
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.path === "sensitivity")).toBe(true);
  });

  it("rejects invalid timestamp", () => {
    const result = validateContextSource({ ...VALID_SOURCE, createdAt: "not-a-date" });
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.path === "createdAt")).toBe(true);
  });

  it("accepts reserved kinds", () => {
    const result = validateContextSource({ ...VALID_SOURCE, kind: "work-event" });
    expect(result.valid).toBe(true);
  });
});

describe("validators — ContextDocument", () => {
  it("accepts a valid document", () => {
    const result = validateContextDocument(VALID_DOCUMENT);
    expect(result.valid).toBe(true);
  });

  it("rejects missing text", () => {
    const result = validateContextDocument({ ...VALID_DOCUMENT, text: "" });
    expect(result.valid).toBe(false);
  });

  it("rejects invalid freshness state", () => {
    const result = validateContextDocument({
      ...VALID_DOCUMENT,
      freshness: { state: "bogus" as never, observedAt: "2026-07-05T03:00:00.000Z" },
    });
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.path === "freshness.state")).toBe(true);
  });

  it("rejects non-JSON-safe metadata", () => {
    const result = validateContextDocument({
      ...VALID_DOCUMENT,
      metadata: { fn: () => "bad" } as never,
    });
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.path === "metadata")).toBe(true);
  });
});

describe("validators — helpers", () => {
  it("isValidIsoTimestamp accepts RFC3339", () => {
    expect(isValidIsoTimestamp("2026-07-05T03:00:00.000Z")).toBe(true);
  });

  it("isValidIsoTimestamp rejects garbage", () => {
    expect(isValidIsoTimestamp("not-a-date")).toBe(false);
    expect(isValidIsoTimestamp(12345 as never)).toBe(false);
  });

  it("isJsonSafe accepts plain JSON", () => {
    expect(isJsonSafe({ a: 1, b: [true, null, "x"] })).toBe(true);
  });

  it("isJsonSafe rejects functions and undefined", () => {
    expect(isJsonSafe({ fn: () => 1 })).toBe(false);
    expect(isJsonSafe(undefined)).toBe(false);
  });
});
