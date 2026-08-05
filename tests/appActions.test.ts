import { describe, expect, it } from "vitest";
import {
  confirmationFor,
  labelForAction,
  riskClassFor,
} from "../src/lib/appActions/types";
import { parseActionIntentsFromToolSteps } from "../src/lib/appActions/dispatcher";
import { enqueuePendingAction, takePendingAction, clearPendingActions } from "../src/lib/appActions/pending";

describe("appActions risk/policy", () => {
  it("classifies recipe as external/hard", () => {
    const a = { type: "runRecipe" as const, recipeId: "cargo-test" };
    expect(riskClassFor(a)).toBe("external");
    expect(confirmationFor(a)).toBe("hard");
  });

  it("classifies createNote as write/soft", () => {
    const a = { type: "createNote" as const, title: "Ideas" };
    expect(riskClassFor(a)).toBe("write");
    expect(confirmationFor(a)).toBe("soft");
  });

  it("labels canvas actions", () => {
    expect(
      labelForAction({ type: "canvasAddNode", label: "A", kind: "machine" }),
    ).toContain("A");
  });
});

describe("pending confirmations", () => {
  it("enqueue and take", () => {
    clearPendingActions();
    const id = enqueuePendingAction(
      { action: { type: "createNote", title: "x" }, source: "voice" },
      "Create note",
      "soft",
    );
    expect(takePendingAction(id)?.label).toBe("Create note");
    expect(takePendingAction(id)).toBeNull();
  });
});

describe("parse app_propose_action", () => {
  it("parses typed proposals", () => {
    const intents = parseActionIntentsFromToolSteps(
      [
        {
          toolName: "app_propose_action",
          ok: true,
          summary: JSON.stringify({
            ok: true,
            appAction: { type: "openCanvas" },
          }),
        },
      ],
      "chat",
    );
    expect(intents[0]?.action).toEqual({ type: "openCanvas" });
  });
});
