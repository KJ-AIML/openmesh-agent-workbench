import { describe, it, expect } from "vitest";
import {
  buildCanonicalRef,
  buildCanonicalIdentity,
  deriveSourceId,
  encodeSourceKey,
  normalizeSeparators,
} from "@/domain/context/canonicalRef";

describe("canonicalRef — identity", () => {
  it("is deterministic for the same inputs", () => {
    const a = buildCanonicalIdentity({
      projectId: "proj-1",
      kind: "doc",
      sourceKey: "architecture/overview.md",
    });
    const b = buildCanonicalIdentity({
      projectId: "proj-1",
      kind: "doc",
      sourceKey: "architecture/overview.md",
    });
    expect(a.canonicalRef).toBe(b.canonicalRef);
    expect(a.id).toBe(b.id);
  });

  it("differs for different projects", () => {
    const a = buildCanonicalRef({ projectId: "p1", kind: "doc", sourceKey: "a.md" });
    const b = buildCanonicalRef({ projectId: "p2", kind: "doc", sourceKey: "a.md" });
    expect(a).not.toBe(b);
  });

  it("differs for different kinds", () => {
    const a = buildCanonicalRef({ projectId: "p1", kind: "doc", sourceKey: "a.md" });
    const b = buildCanonicalRef({ projectId: "p1", kind: "note", sourceKey: "a.md" });
    expect(a).not.toBe(b);
  });

  it("formats with openmesh:// scheme and project/kind/key segments", () => {
    const ref = buildCanonicalRef({
      projectId: "proj-alpha",
      kind: "task",
      sourceKey: "task-123",
    });
    expect(ref).toContain("openmesh://project/proj-alpha/task/task-123");
  });

  it("normalizes Windows path separators in sourceKey", () => {
    const ref = buildCanonicalRef({
      projectId: "p1",
      kind: "doc",
      sourceKey: "architecture\\overview.md",
    });
    expect(ref).toContain("architecture/overview.md");
    expect(ref).not.toContain("\\");
  });

  it("handles nested doc paths deterministically", () => {
    const a = buildCanonicalIdentity({
      projectId: "p1",
      kind: "doc",
      sourceKey: "engineering/backend/api.md",
    });
    const b = buildCanonicalIdentity({
      projectId: "p1",
      kind: "doc",
      sourceKey: "engineering/backend/api.md",
    });
    expect(a.id).toBe(b.id);
  });

  it("handles spaces and unicode in sourceKey", () => {
    const ref = buildCanonicalRef({
      projectId: "p1",
      kind: "note",
      sourceKey: "Tilen Maës notes.md",
    });
    expect(ref).toBeTruthy();
    expect(() => new URL(ref.replace("openmesh", "https"))).not.toThrow();
  });

  it("is idempotent on repeated derivation", () => {
    const canonicalRef = buildCanonicalRef({
      projectId: "p1",
      kind: "doc",
      sourceKey: "a.md",
    });
    const id1 = deriveSourceId(canonicalRef);
    const id2 = deriveSourceId(canonicalRef);
    expect(id1).toBe(id2);
  });

  it("throws on empty required inputs", () => {
    expect(() => buildCanonicalRef({ projectId: "", kind: "doc", sourceKey: "a" })).toThrow();
    expect(() =>
      buildCanonicalRef({ projectId: "p", kind: "" as never, sourceKey: "a" })
    ).toThrow();
    expect(() => buildCanonicalRef({ projectId: "p", kind: "doc", sourceKey: "" })).toThrow();
  });
});
