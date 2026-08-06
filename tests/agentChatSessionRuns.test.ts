import { describe, expect, it } from "vitest";
import {
  completeRunningOfKind,
  completeSessionRun,
  countRunning,
  createSessionRun,
  formatElapsed,
  listTerminalRuns,
  looksLikeTerminalTool,
  sessionHasCanvasSignal,
  truncateCommand,
  upsertSessionRun,
} from "@/lib/agentChat/sessionRuns";

describe("sessionRuns helpers", () => {
  it("recognizes verify/delegate-style terminal tools", () => {
    expect(looksLikeTerminalTool("verify")).toBe(true);
    expect(looksLikeTerminalTool("Verify recipe")).toBe(true);
    expect(looksLikeTerminalTool("delegate")).toBe(true);
    expect(looksLikeTerminalTool("Delegate to CLI")).toBe(true);
    expect(looksLikeTerminalTool("pilot_status")).toBe(false);
    expect(looksLikeTerminalTool("read_file")).toBe(false);
  });

  it("truncates and formats elapsed time", () => {
    expect(truncateCommand("a".repeat(60), 20).endsWith("…")).toBe(true);
    expect(truncateCommand("short cmd", 52)).toBe("short cmd");
    expect(formatElapsed(0, 12_000)).toBe("12s");
    expect(formatElapsed(0, 125_000)).toBe("2m 05s");
    expect(formatElapsed(0, 90_000, 30_000)).toBe("30s");
  });

  it("upserts and completes runs; counts running terminals", () => {
    let runs = [
      createSessionRun({
        id: "working:1",
        kind: "working",
        title: "Working",
        command: "Thinking…",
      }),
    ];
    runs = upsertSessionRun(
      runs,
      createSessionRun({
        id: "term:v1",
        kind: "terminal",
        title: "Verify",
        command: "typecheck",
        toolId: "verify",
      }),
    );
    expect(countRunning(runs, "working")).toBe(1);
    expect(countRunning(runs, "terminal")).toBe(1);

    runs = completeSessionRun(runs, "term:v1", {
      status: "done",
      output: "ok",
      messageId: "msg-1",
    });
    expect(countRunning(runs, "terminal")).toBe(0);
    expect(listTerminalRuns(runs)[0]?.output).toBe("ok");
    expect(listTerminalRuns(runs)[0]?.messageId).toBe("msg-1");

    runs = completeRunningOfKind(runs, "working", "cancelled");
    expect(countRunning(runs, "working")).toBe(0);
    expect(runs.find((r) => r.id === "working:1")?.status).toBe("cancelled");
  });

  it("detects canvas signals in transcript text/tools", () => {
    expect(
      sessionHasCanvasSignal([
        { text: "hello", toolCalls: [{ toolId: "pilot_status", summary: "ok" }] },
      ]),
    ).toBe(false);
    expect(
      sessionHasCanvasSignal([
        {
          text: "```canvas\n{\"schema\":\"openmesh.canvas/1\"}\n```",
        },
      ]),
    ).toBe(true);
    expect(
      sessionHasCanvasSignal([
        {
          text: "done",
          toolCalls: [
            { toolId: "canvas_upsert_auto_ui", summary: "saved auto ui" },
          ],
        },
      ]),
    ).toBe(true);
  });
});
