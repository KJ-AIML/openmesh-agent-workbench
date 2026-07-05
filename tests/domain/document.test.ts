import { describe, it, expect } from "vitest";
import { createContextDocument } from "@/domain/context/documentBuilder";
import { mapTaskSource } from "@/domain/context/mappers";
import type { ContextSource } from "@/domain/context/types";

const SOURCE: ContextSource = mapTaskSource(
  { projectId: "p-1", taskId: "t-1", title: "Fix the deploy flow" },
  { now: () => new Date("2026-07-05T03:00:00.000Z") }
);

describe("createContextDocument", () => {
  it("builds a document from source + supplied text (no I/O)", () => {
    const doc = createContextDocument(SOURCE, "Title: Fix deploy flow\nStatus: pending\n", {
      freshnessState: "fresh",
    });
    expect(doc.kind).toBe("task");
    expect(doc.projectId).toBe("p-1");
    expect(doc.text).toContain("Fix deploy flow");
    expect(doc.freshness.state).toBe("fresh");
    expect(doc.id).not.toBe(SOURCE.id);
  });

  it("JSON serializes cleanly", () => {
    const doc = createContextDocument(SOURCE, "plain text", {
      metadata: { pages: 1, ext: "md" },
    });
    const json = JSON.stringify(doc);
    const parsed = JSON.parse(json);
    expect(parsed.title).toBe("Fix the deploy flow");
    expect(parsed.metadata.pages).toBe(1);
  });

  it("preserves sensitive metadata but does not fabricate data", () => {
    const doc = createContextDocument(SOURCE, "x", { sensitivity: "secret" });
    expect(doc.sensitivity).toBe("secret");
  });
});
