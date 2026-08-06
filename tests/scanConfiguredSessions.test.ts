import { describe, it, expect, vi, beforeEach } from "vitest";

const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

import {
  pickRecentAgentSessions,
  scanConfiguredSessionsResult,
} from "@/lib/scanConfiguredSessions";
import type { ScannedSession } from "@/lib/adapters/types";

function session(
  partial: Partial<ScannedSession> & Pick<ScannedSession, "id" | "lastActiveAt">,
): ScannedSession {
  return {
    toolName: "codex",
    title: partial.id,
    sessionPath: `/tmp/${partial.id}.jsonl`,
    fileName: `${partial.id}.jsonl`,
    createdAt: partial.lastActiveAt,
    fileSizeBytes: 1,
    isReal: true,
    ...partial,
  };
}

describe("pickRecentAgentSessions", () => {
  it("returns most recent sessions first up to limit", () => {
    const picked = pickRecentAgentSessions(
      [
        session({ id: "old", lastActiveAt: "2026-01-01T00:00:00.000Z" }),
        session({ id: "new", lastActiveAt: "2026-06-01T00:00:00.000Z" }),
        session({ id: "mid", lastActiveAt: "2026-03-01T00:00:00.000Z" }),
      ],
      2,
    );
    expect(picked.map((s) => s.id)).toEqual(["new", "mid"]);
  });

  it("returns empty for empty input", () => {
    expect(pickRecentAgentSessions([], 4)).toEqual([]);
  });
});

describe("scanConfiguredSessionsResult", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("returns ok sessions from workspace scan", async () => {
    const sessions = [
      session({ id: "s1", lastActiveAt: "2026-06-01T00:00:00.000Z" }),
    ];
    invoke.mockResolvedValue({ success: true, sessions });

    const result = await scanConfiguredSessionsResult(
      undefined,
      100,
      "/tmp/demo",
    );

    expect(result).toEqual({ ok: true, sessions });
    expect(invoke).toHaveBeenCalledWith("scan_workspace_agent_sessions", {
      workspaceCwd: "/tmp/demo",
      limit: 100,
      overrides: {},
    });
  });

  it("returns error when backend reports failure", async () => {
    invoke.mockResolvedValue({
      success: false,
      sessions: [],
      error: "permission denied",
    });

    const result = await scanConfiguredSessionsResult(
      undefined,
      10,
      "/tmp/demo",
    );

    expect(result).toEqual({
      ok: false,
      sessions: [],
      error: "permission denied",
    });
  });

  it("returns empty ok when no workspace is open", async () => {
    const result = await scanConfiguredSessionsResult(undefined, 10, null);
    expect(result).toEqual({ ok: true, sessions: [] });
    expect(invoke).not.toHaveBeenCalled();
  });
});
