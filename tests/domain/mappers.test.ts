import { describe, it, expect } from "vitest";
import {
  mapDocSource,
  mapNoteSource,
  mapSnapshotSource,
  mapTaskSource,
  mapRecentSource,
  mapAgentSessionSource,
} from "@/domain/context/mappers";
import { DEFAULT_AGENT_CONTEXT_ENABLED, DEFAULT_SENSITIVITY } from "@/domain/context/types";

const FIXED_CLOCK = () => new Date("2026-07-05T03:00:00.000Z");

describe("mappers — all current kinds produce valid ContextSource shape", () => {
  it("maps doc", () => {
    const s = mapDocSource(
      { projectId: "p1", relativePath: "architecture/overview.md", title: "Arch" },
      { now: FIXED_CLOCK }
    );
    expect(s.kind).toBe("doc");
    expect(s.projectId).toBe("p1");
    expect(s.title).toBe("Arch");
    expect(s.schemaVersion).toBe("1.0.0");
    expect(s.canonicalRef).toContain("doc/architecture/overview.md");
    expect(s.createdAt).toBe("2026-07-05T03:00:00.000Z");
    expect(s.updatedAt).toBe("2026-07-05T03:00:00.000Z");
  });

  it("maps note", () => {
    const s = mapNoteSource({ projectId: "p1", filename: "daily" }, { now: FIXED_CLOCK });
    expect(s.kind).toBe("note");
    expect(s.canonicalRef).toContain("note/daily");
  });

  it("maps snapshot", () => {
    const s = mapSnapshotSource(
      { projectId: "p1", filename: "snap.md" },
      { now: FIXED_CLOCK }
    );
    expect(s.kind).toBe("snapshot");
    expect(s.canonicalRef).toContain("notes/snapshots");
  });

  it("maps task", () => {
    const s = mapTaskSource(
      { projectId: "p1", taskId: "task-123", title: "Do thing" },
      { now: FIXED_CLOCK }
    );
    expect(s.kind).toBe("task");
    expect(s.canonicalRef).toContain("task/task-123");
  });

  it("maps recent (transitional)", () => {
    const s = mapRecentSource(
      { projectId: "p1", recentId: "r-1", title: "Recent X" },
      { now: FIXED_CLOCK }
    );
    expect(s.kind).toBe("recent");
  });

  it("maps agent session", () => {
    const s = mapAgentSessionSource(
      { projectId: "p1", sessionId: "sess-9", title: "Session" },
      { now: FIXED_CLOCK }
    );
    expect(s.kind).toBe("agent-session");
  });
});

describe("mappers — identity derived from canonical ref", () => {
  it("doc ID matches sourceId derivation", () => {
    const s = mapDocSource(
      { projectId: "p1", relativePath: "a.md" },
      { now: FIXED_CLOCK }
    );
    expect(s.id).toBeTruthy();
    expect(s.id.length).toBe(16);
  });

  it("same canonical ref yields same ID across calls", () => {
    const a = mapDocSource({ projectId: "p1", relativePath: "a.md" }, { now: FIXED_CLOCK });
    const b = mapDocSource({ projectId: "p1", relativePath: "a.md" }, { now: FIXED_CLOCK });
    expect(a.id).toBe(b.id);
  });
});

describe("mappers — deterministic observedAt with injected clock", () => {
  it("uses injected clock instead of wall time", () => {
    const s = mapTaskSource(
      { projectId: "p1", taskId: "t", title: "x" },
      { now: FIXED_CLOCK }
    );
    expect(s.createdAt).toBe("2026-07-05T03:00:00.000Z");
    expect(s.updatedAt).toBe("2026-07-05T03:00:00.000Z");
  });
});

describe("mappers — privacy defaults", () => {
  it("defaults sensitivity to private", () => {
    const s = mapDocSource(
      { projectId: "p1", relativePath: "a.md" },
      { now: FIXED_CLOCK }
    );
    expect(s.sensitivity).toBe(DEFAULT_SENSITIVITY);
  });

  it("defaults agentContextEnabled to false (fail closed)", () => {
    const s = mapDocSource(
      { projectId: "p1", relativePath: "a.md" },
      { now: FIXED_CLOCK }
    );
    expect(s.agentContextEnabled).toBe(DEFAULT_AGENT_CONTEXT_ENABLED);
  });

  it("respects explicit agentContextEnabled", () => {
    const s = mapDocSource(
      { projectId: "p1", relativePath: "a.md", agentContextEnabled: true },
      { now: FIXED_CLOCK }
    );
    expect(s.agentContextEnabled).toBe(true);
  });
});

describe("mappers — ownerPersonId", () => {
  it("remains optional for doc", () => {
    const s = mapDocSource(
      { projectId: "p1", relativePath: "a.md" },
      { now: FIXED_CLOCK }
    );
    expect(s.ownerPersonId).toBeUndefined();
  });

  it("uses options ownerPersonId", () => {
    const s = mapDocSource(
      { projectId: "p1", relativePath: "a.md" },
      { ownerPersonId: "ter", now: FIXED_CLOCK }
    );
    expect(s.ownerPersonId).toBe("ter");
  });
});

describe("mappers — input validation", () => {
  it("throws on empty required fields", () => {
    expect(() => mapDocSource({ projectId: "", relativePath: "a.md" })).toThrow();
    expect(() => mapNoteSource({ projectId: "p", filename: "" })).toThrow();
    expect(() => mapTaskSource({ projectId: "p", taskId: "", title: "x" })).toThrow();
  });
});
