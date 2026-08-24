import { test as base, expect } from "@playwright/test";
import { installMockTauri, waitForShell } from "./support/mockTauri";

type OpenMeshFixtures = {
  openMesh: (
    path: string,
    options?: {
      mockTauri?: boolean;
      scenario?:
        | "seeded"
        | "empty"
        | "patch"
        | "slow-chat"
        | "oauth-success"
        | "oauth-error"
        | "runtime-unavailable"
        | "scan-error"
        | "chat-not-ready"
        | "chat-error"
        | "chat-rich"
        | "chat-invalid-rich"
        | "chat-tools"
        | "chat-tool-error"
        | "chat-progress"
        | "provider-error"
        | "provider-empty"
        | "provider-mutation-error"
        | "runtime-error"
        | "runtime-empty"
        | "runtime-unauthorized"
        | "runtime-chat-unauthorized"
        | "runtime-unsupported"
        | "agent-provider-error"
        | "agent-secret-error"
        | "project-error"
        | "mixed-sessions"
        | "transcript-error"
        | "extensions-seeded"
        | "command-error"
        | "path-invalid"
        | "context-empty"
        | "context-degraded"
        | "context-partial"
        | "context-error"
        | "context-invalid-ref"
        | "docs-empty"
        | "notes-empty"
        | "oauth-admin-empty"
        | "oauth-model-empty"
        | "pty-error"
        | "home-action-error"
        | "update-assets-missing"
        | "update-unsupported"
        | "update-open-failure"
        | "continuity-error";
      runtime?: "tauri" | "web";
    },
  ) => Promise<void>;
};

export const test = base.extend<OpenMeshFixtures>({
  openMesh: async ({ page }, use) => {
    await use(async (path, options = {}) => {
      if (options.mockTauri) await installMockTauri(page, options);
      await page.goto(path);
      await waitForShell(page);
    });
  },
});

export { expect, installMockTauri, waitForShell };
