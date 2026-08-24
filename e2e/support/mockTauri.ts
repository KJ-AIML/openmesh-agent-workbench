import { expect, type Page } from "@playwright/test";

export async function waitForShell(page: Page) {
  await page.waitForLoadState("domcontentloaded");
  await expect(page.locator(".shell")).toBeVisible({ timeout: 20_000 });
  await expect(page.locator(".startup-splash")).toHaveCount(0, {
    timeout: 15_000,
  });
}

export async function installMockTauri(
  page: Page,
  options: {
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
  } = {},
) {
  await page.addInitScript(
    ({ scenario, runtime }) => {
      const target = window as unknown as Record<string, unknown>;
      const callbacks = new Map<number, (payload: unknown) => void>();
      const eventListeners = new Map<string, Set<number>>();
      let nextCallbackId = 1;
      let sequence = 1;
      const now = "2026-08-22T00:00:00.000Z";
      const resetMarker = sessionStorage.getItem("openmesh.e2e.reset") === "1";
      const effectiveEmpty = scenario === "empty" || resetMarker;
      let secretConfigured = false;
      let extensionInventory: any = { skills: [], hooks: [], plugins: [] };
      let extensionCatalog: any[] = [];
      let patchStatus: "proposed" | "applied" | "rejected" | "rolled_back" =
        "proposed";
      let oauthManagementPort = 8317;
      let oauthSecretConfigured = true;
      let oauthSidecarClientKeyConfigured = true;
      let oauthSidecarEnabled = true;
      let oauthPollCount = 0;
      let oauthAdminRevision = "r1-0123456789abcdef";
      let oauthAuthFiles: any[] = [
        {
          name: "claude.json",
          provider: "claude",
          enabled: true,
          runtimeOnly: false,
          unavailable: false,
          priority: 0,
          sizeBytes: 42,
          updatedAt: now,
          metadataWritable: true,
        },
        {
          name: "runtime.json",
          provider: "codex",
          enabled: true,
          runtimeOnly: true,
          unavailable: false,
          priority: 0,
          metadataWritable: false,
        },
        {
          name: "unavailable.json",
          provider: "xai",
          enabled: true,
          runtimeOnly: false,
          unavailable: true,
          priority: 0,
          metadataWritable: false,
        },
      ];
      let providerRevisionCounter = 1;
      const nextId = (prefix: string) => `${prefix}-${sequence++}`;
      const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value));

      target.__TAURI__ = {};
      if (runtime === "web") delete target.__TAURI__;
      target.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
        unregisterListener: (event: string, callbackId: number) => {
          const listeners = eventListeners.get(event);
          listeners?.delete(callbackId);
          callbacks.delete(callbackId);
        },
      };

      const project = {
        id: "browser-project",
        name: "Browser Project",
        folderPath: "/tmp/browser-project",
        defaultBranch: "main",
        sprintSource: "none",
        status: "active",
        createdAt: now,
        updatedAt: now,
      };
      const projects = new Map<string, any>(
        effectiveEmpty ? [] : [[project.folderPath, project]],
      );
      let currentProjectPath: string | null = effectiveEmpty
        ? null
        : project.folderPath;
      const defaultSettings: any = {
        workspace: { theme: "dark" },
        provider: {
          name: "",
          apiKeyConfigured: false,
          usageTrackingEnabled: false,
          defaultModel: "sidecar-model",
        },
        models: { localModelEnabled: false },
        server: {
          mode: "local",
          apiBaseUrl: "",
          healthStatus: "unknown",
          syncStatus: "unknown",
        },
        agentClis: {},
        sessionDirs: {
          codexEnabled: false,
          claudeCodeEnabled: false,
          opencodeEnabled: false,
          cursorEnabled: false,
          geminiEnabled: false,
          grokEnabled: false,
        },
        localPaths: {},
        appearance: {
          theme: "dark",
          fontSize: "medium",
          density: "comfortable",
          topNavbarTabs: { chat: true, work: true, docs: true, sprint: true },
        },
        extensions: { skills: {}, hooks: {}, plugins: {} },
        voice: { sttModel: "openai/whisper-large-v3", sttLanguage: "" },
        oauth: { managementPort: 8317, sidecarEnabled: true },
      };
      const settings: any = (() => {
        try {
          const raw = sessionStorage.getItem("openmesh.e2e.settings");
          return raw ? JSON.parse(raw) : clone(defaultSettings);
        } catch {
          return clone(defaultSettings);
        }
      })();

      if (scenario === "chat-not-ready") {
        settings.oauth.sidecarEnabled = false;
        settings.provider.defaultModel = "";
        settings.provider.apiKeyConfigured = false;
      }

      if (scenario === "extensions-seeded") {
        extensionInventory = {
          skills: [
            {
              id: "browser-skill",
              name: "Browser Skill",
              description: "Seeded browser skill",
              body: "Use the browser fixture safely.",
              enabled: true,
              source: "project",
              pluginId: null,
              path: "/tmp/browser-skill/SKILL.md",
            },
          ],
          hooks: [
            {
              id: "browser-hook",
              event: "on_chat_start",
              appendContext: "Browser hook context",
              command: null,
              enabled: true,
              source: "project",
              pluginId: null,
            },
          ],
          plugins: [
            {
              id: "browser-plugin",
              name: "Browser Plugin",
              version: "1.0.0",
              description: "Seeded browser plugin",
              enabled: true,
              source: "project",
              path: "/tmp/browser-plugin/openmesh.plugin.json",
              skillIds: ["browser-skill"],
              hookIds: ["browser-hook"],
            },
          ],
        };
        extensionCatalog = [
          {
            id: "browser-catalog",
            kind: "skill",
            name: "Browser Catalog Skill",
            description: "A seeded local catalog entry",
            installed: false,
          },
        ];
      }

      const projectData = new Map<string, any>();
      const ptyCommands: string[] = [];
      const boardSceneSaves: unknown[] = [];
      const ensureProjectData = (path: string) => {
        const projectRecord = projects.get(path) ?? project;
        if (!projectData.has(path)) {
          projectData.set(path, {
            sprint: null,
            tasks: [],
            recent: [],
            sessions: [
              {
                id: "saved-session-1",
                tool: "codex",
                title: "Saved browser session",
                projectId: projectRecord.id,
                sourcePath: `${path}/.codex/session.jsonl`,
                status: "active",
                summary: "A saved session for browser coverage.",
                startedAt: now,
                lastActiveAt: now,
                changedFiles: ["src/App.vue"],
                isImportant: false,
                createdAt: now,
                updatedAt: now,
              },
            ],
            presets: [],
            docs: new Map<string, string>(
              scenario === "docs-empty"
                ? []
                : [
                    [
                      "README.md",
                      "# Browser project\n\nA document used by browser tests.",
                    ],
                    ["guides/flow.md", "# Flow\n\nThe browser feature flow."],
                  ],
            ),
            notes: new Map<string, string>(
              scenario === "notes-empty"
                ? []
                : [["welcome.md", "# Welcome\n\nA note used by browser tests."]],
            ),
            canvases: [
              {
                id: "canvas-1",
                title: "Agent network",
                schemaVersion: "1",
                nodes: [
                  {
                    id: "node-1",
                    label: "Browser Project",
                    kind: "project",
                    x: 80,
                    y: 80,
                  },
                ],
                edges: [],
                updatedAt: Date.parse(now),
              },
            ],
            autoUi: [
              {
                schema: "openmesh.canvas/1",
                id: "auto-ui-1",
                title: "Browser overview",
                summary: "A deterministic Auto UI artifact.",
                blocks: [
                  { type: "h1", text: "Browser overview" },
                  { type: "stat", label: "Coverage", value: "All routes" },
                ],
                updatedAt: Date.parse(now),
              },
            ],
            boards: [
              {
                schema: "openmesh.board/1",
                id: "board-1",
                title: "Browser board",
                engine: "excalidraw",
                scene: { elements: [], appState: {}, files: {} },
                updatedAt: Date.parse(now),
              },
            ],
            continuity: {
              peers: [],
              team: null,
              trust: null,
              onlineProxy: null,
              lanRunning: false,
              chat: [],
            },
          });
        }
        const record = projectData.get(path)!;
        if (scenario === "chat-tools" && !record.chatToolsSeeded) {
          record.continuity.onlineProxy = {
            protocolVersion: "1",
            proxyId: "proxy-browser",
            workspaceId: "workspace-browser",
            ownerLabel: projectRecord.name,
            mode: "local-scaffold",
            defaultFreshnessTier: "standard",
            useRelayReceived: true,
            createdAt: now,
            updatedAt: now,
          };
          record.continuity.team = {
            protocolVersion: "1",
            teamId: "team-browser",
            displayName: "Browser team",
            hostWorkspaceId: "workspace-browser",
            members: [
              {
                memberId: "member-owner",
                label: projectRecord.name,
                role: "owner",
                joinedAt: now,
              },
            ],
            limitations: [],
          };
          record.continuity.trust = {
            protocolVersion: "1",
            teamId: "team-browser",
            remoteQueryEnabled: true,
            queryAllowlistMode: "allowlist-only",
            queryAllowlist: [],
            secretTopicsFailClosed: true,
            allowSecretExport: false,
            syncRequireSelective: true,
            adminMemberIds: ["member-owner"],
            limitations: [],
          };
          record.chatToolsSeeded = true;
        }
        if (scenario === "mixed-sessions" && !record.mixedSeeded) {
          record.sessions = [
            "codex",
            "claude",
            "opencode",
            "cursor",
            "gemini",
            "grok",
          ].map((tool, index) => ({
            id: `saved-${tool}-session`,
            tool,
            title: `Saved ${tool} session`,
            projectId: projectRecord.id,
            sourcePath: `${path}/.${tool}/session.jsonl`,
            status: "active",
            summary: `A saved ${tool} session for browser coverage.`,
            startedAt: now,
            lastActiveAt: now,
            changedFiles: [`src/${tool}.ts`],
            isImportant: false,
            createdAt: now,
            updatedAt: now,
            order: index,
          }));
          record.mixedSeeded = true;
        }
        return projectData.get(path)!;
      };
      const dataFor = (path?: string) =>
        ensureProjectData(path || currentProjectPath || project.folderPath);
      const fileEntry = (name: string, content: string) => ({
        name,
        path: name,
        is_dir: false,
        size: content.length,
        modified_at: now,
      });
      const docsTree = (docs: Map<string, string>) => {
        const root: any[] = [];
        for (const [path, content] of docs) {
          const parts = path.split("/");
          let cursor = root;
          let prefix = "";
          parts.forEach((part, index) => {
            prefix = prefix ? `${prefix}/${part}` : part;
            const isFile = index === parts.length - 1;
            let node = cursor.find((entry) => entry.name === part);
            if (!node) {
              node = isFile
                ? {
                    name: part,
                    path: prefix,
                    nodeType: "file",
                    size: content.length,
                    modifiedAt: now,
                  }
                : {
                    name: part,
                    path: prefix,
                    nodeType: "folder",
                    children: [],
                  };
              cursor.push(node);
            }
            if (!isFile) cursor = node.children;
          });
        }
        return root;
      };

      const providerCatalog: any = {
        revision: "r1-0123456789abcdef",
        routingStrategy: "round-robin",
        providers: [
          {
            section: "openai-compatibility",
            sectionLabel: "OpenAI compatible",
            index: 0,
            name: "browser-gateway-a",
            baseUrl: "https://api.example.com/v1",
            endpointConfigured: true,
            models: [{ name: "browser-model" }],
            priority: 10,
            enabled: true,
            keyConfigured: true,
            keyCount: 1,
            headerCount: 0,
          },
          {
            section: "openai-compatibility",
            sectionLabel: "OpenAI compatible",
            index: 1,
            name: "browser-gateway-b",
            baseUrl: "https://backup.example.com/v1",
            endpointConfigured: true,
            models: [],
            priority: 20,
            enabled: true,
            keyConfigured: true,
            keyCount: 1,
            headerCount: 0,
          },
        ],
      };
      if (scenario === "provider-empty") providerCatalog.providers = [];
      const providerCommands: string[] = [];
      const nextRevision = () =>
        `r1-${String(++providerRevisionCounter).padStart(16, "0")}`;
      const refreshProviderIndexes = () => {
        const indexes = new Map<string, number>();
        providerCatalog.providers = providerCatalog.providers.map(
          (provider: any) => {
            const index = indexes.get(provider.section) ?? 0;
            indexes.set(provider.section, index + 1);
            return { ...provider, index };
          },
        );
        providerCatalog.revision = nextRevision();
      };
      target.__OPENMESH_PROVIDER_COMMANDS__ = providerCommands;
      target.__OPENMESH_PTY_COMMANDS__ = ptyCommands;
      target.__OPENMESH_BOARD_SCENE_SAVES__ = boardSceneSaves;
      target.__OPENMESH_EMIT_EVENT__ = (event: string, payload: unknown) => {
        for (const callbackId of eventListeners.get(event) ?? []) {
          callbacks.get(callbackId)?.({ event, payload, id: callbackId });
        }
      };
      const oauthCommands: string[] = [];
      const runtimeCommands: string[] = [];
      target.__OPENMESH_OAUTH_COMMANDS__ = oauthCommands;
      target.__OPENMESH_RUNTIME_COMMANDS__ = runtimeCommands;

      const contextResult = {
        document_id: "context-doc-1",
        source_id: "README.md",
        source_kind: "doc",
        project_id: project.id,
        canonical_ref: `openmesh://project/${project.id}/doc/README.md`,
        title: "README.md",
        snippet: "A document used by browser tests.",
        sensitivity: "normal",
        freshness_state: "fresh",
        observed_at: now,
        source_updated_at: now,
      };
      const contextInspection = {
        ...contextResult,
        text: "# Browser project\n\nA document used by browser tests.",
        agent_context_enabled: true,
        indexed_at: now,
      };

      const continuityView = (path: string) => {
        const continuity = dataFor(path).continuity;
        return {
          summary: {
            openPendingCount: 1,
            peerCount: continuity.peers.length,
            envelopeCount: 1,
            auditEventCount: continuity.trust ? 1 : 0,
            onlineProxyInitialized: !!continuity.onlineProxy,
          },
          pending: {
            workspaceId: "workspace-browser",
            generatedAt: now,
            protocolVersion: "1",
            items: [
              {
                id: "pending-1",
                summary: "Review browser continuity",
                source: "continuity-attention",
                sourceId: "attention-1",
                status: "open",
                severity: "medium",
                createdAt: now,
                reason: "Browser fixture pending item",
              },
            ],
            openCount: 1,
            sourceCounts: {
              proxyPending: 0,
              continuityAttention: 1,
              unresolvedSignal: 0,
            },
            limitations: ["Browser fixture is read-only."],
          },
          digest: {
            workspaceId: "workspace-browser",
            generatedAt: now,
            protocolVersion: "1",
            window: { since: now, until: now },
            summary: "Browser continuity digest",
            needsMe: [],
            whatIMissed: {
              completed: ["A completed item"],
              changed: [],
              blocked: [],
              decided: [],
              needsAttention: [],
              stillOpen: [],
            },
            catchUpSummary: "One completed item is available.",
            handoffs: [],
            evidenceRefs: [],
            limitations: [],
          },
        };
      };
      const trustView = (path: string) => {
        const continuity = dataFor(path).continuity;
        return (
          continuity.trust || {
            protocolVersion: "1",
            teamId: continuity.team?.teamId || "team-browser",
            remoteQueryEnabled: false,
            queryAllowlistMode: "deny-all",
            queryAllowlist: [],
            secretTopicsFailClosed: true,
            allowSecretExport: false,
            syncRequireSelective: true,
            adminMemberIds: [],
            limitations: ["Browser fixture only."],
          }
        );
      };
      const onlineAnswer = (question: string) => ({
        protocolVersion: "1",
        answerId: nextId("answer"),
        proxyId: "proxy-browser",
        workspaceId: "workspace-browser",
        question,
        answerText: "Browser continuity answer",
        generatedAt: now,
        freshness: {
          statement: "Fresh fixture",
          evaluatedAt: now,
          tier: "standard",
          isSufficient: true,
          confidenceLabel: "high",
          oldestEvidenceAgeSeconds: 1,
          staleWarnings: [],
          evidenceSourceIds: ["fixture-1"],
        },
        refused: false,
        mode: "local-scaffold",
        liveEngine: false,
      });
      const runtimeStatus = {
        mode: "attach-only",
        ownership: "external-sidecar",
        managementPort: 8317,
        managementEndpoint: "http://127.0.0.1:8317/v0/management/",
        managementStatus: "ready",
        managementSecretConfigured: true,
        managementError: null,
        dataPlaneEndpoint: "http://127.0.0.1:8317/v1",
        dataPlaneStatus: "ready",
        sidecarClientKeyConfigured: true,
        dataPlaneError: null,
        configurationStatus: "available",
        configurationError: null,
        configuration: {
          port: 8317,
          allowLan: false,
          routingStrategy: "round-robin",
          sessionAffinity: true,
          sessionAffinityTtl: "10m",
          requestRetry: 3,
          maxRetryCredentials: 2,
          maxRetryInterval: 30,
          streamingBootstrapRetries: 1,
          pluginsEnabled: true,
          apiKeyCount: 2,
          proxyConfigured: false,
        },
        providers: [
          {
            provider: "codex",
            total: 2,
            enabled: 2,
            disabled: 0,
            unavailable: 0,
            runtimeOnly: 0,
          },
        ],
        capabilities: {
          oauthLogin: "available",
          accountSummary: "read-only",
          modelCatalog: "read-only",
          chatDataPlane: "available",
          providerConfiguration: "available",
          authFileWrites: "available",
          quotas: "deferred",
          usageAnalytics: "deferred",
          coreLifecycle: "deferred",
        },
      };

      const lanPresence = (address: string) => ({
        address,
        state: "live",
        probedAt: now,
        latencyMs: 4,
        health: {
          ok: true,
          protocol: "1",
          peerId: "peer-browser",
          ownerLabel: "Browser peer",
          projectId: "remote-project",
          httpPort: 41778,
        },
        lastSeenAt: now,
      });

      target.__TAURI_INTERNALS__ = {
        transformCallback(callback: (payload: unknown) => void) {
          const id = nextCallbackId++;
          callbacks.set(id, callback);
          return id;
        },
        unregisterCallback(id: number) {
          callbacks.delete(id);
        },
        invoke: async (command: string, args: any = {}) => {
          if (command.startsWith("cliproxy_")) providerCommands.push(command);
          if (command.startsWith("oauth_")) oauthCommands.push(command);
          if (command === "oauth_runtime_status") runtimeCommands.push(command);
          const path =
            args.projectPath || currentProjectPath || project.folderPath;
          const data = dataFor(path);
          const current = projects.get(path) || project;

          switch (command) {
            case "get_host_os":
              return scenario === "update-unsupported" ? "freebsd" : "macos";
            case "get_host_arch":
              return "aarch64";
            case "validate_path":
              if (scenario === "path-invalid") {
                return {
                  exists: false,
                  isDirectory: false,
                  isFile: false,
                  normalizedPath: args.path || "",
                };
              }
              return {
                exists: true,
                isDirectory: true,
                isFile: false,
                normalizedPath: args.path || "",
              };
            case "get_git_status":
              if (scenario === "home-action-error") {
                return {
                  success: false,
                  is_repo: false,
                  branch: null,
                  dirty_count: 0,
                  staged_count: 0,
                  untracked_count: 0,
                  last_commit_hash: null,
                  last_commit_message: null,
                  error: "git status unavailable",
                };
              }
              return {
                success: true,
                is_repo: true,
                branch: "main",
                dirty_count: 0,
                staged_count: 0,
                untracked_count: 0,
                last_commit_hash: "a1b2c3d",
                last_commit_message: "Initial commit",
                error: null,
              };
            case "oauth_config_status":
              return {
                managementPort: oauthManagementPort,
                endpoint: `http://127.0.0.1:${oauthManagementPort}/v0/management`,
                dataPlaneEndpoint: `http://127.0.0.1:${oauthManagementPort}/v1`,
                sidecarEnabled: oauthSidecarEnabled,
                secretConfigured: oauthSecretConfigured,
                sidecarClientKeyConfigured: oauthSidecarClientKeyConfigured,
              };
            case "get_settings":
              return clone(settings);
            case "save_settings":
              Object.assign(settings, clone(args.settings || {}));
              try {
                sessionStorage.setItem(
                  "openmesh.e2e.settings",
                  JSON.stringify(settings),
                );
              } catch {
                // Browser fixture persistence is best effort.
              }
              if (args.settings?.oauth) {
                oauthManagementPort =
                  Number(args.settings.oauth.managementPort) ||
                  oauthManagementPort;
                oauthSidecarEnabled = !!args.settings.oauth.sidecarEnabled;
              }
              return null;
            case "get_projects_list":
              return [...projects.keys()];
            case "add_project_to_list":
              if (!projects.has(args.path)) {
                projects.set(args.path, {
                  ...project,
                  id: nextId("project"),
                  name: args.path.split("/").pop() || "New Project",
                  folderPath: args.path,
                  createdAt: now,
                  updatedAt: now,
                });
                ensureProjectData(args.path);
              }
              return null;
            case "remove_project_from_list":
              projects.delete(args.path);
              return null;
            case "get_app_state":
              return { currentProjectId: currentProjectPath };
            case "save_app_state":
              currentProjectPath = args.state?.currentProjectId ?? null;
              return null;
            case "init_project_cmd":
              if (scenario === "project-error")
                throw new Error("project initialization failed");
              projects.set(
                args.projectPath,
                projects.get(args.projectPath) || {
                  ...project,
                  id: nextId("project"),
                  name: args.projectPath.split("/").pop() || "New Project",
                  folderPath: args.projectPath,
                  createdAt: now,
                  updatedAt: now,
                },
              );
              ensureProjectData(args.projectPath);
              return null;
            case "get_project":
              return projects.get(args.projectPath) || null;
            case "save_project":
              projects.set(args.projectPath, clone(args.project));
              currentProjectPath = args.projectPath;
              ensureProjectData(args.projectPath);
              return null;
            case "delete_project_cmd":
              projects.delete(path);
              projectData.delete(path);
              if (currentProjectPath === path) currentProjectPath = null;
              return null;
            case "get_sessions":
              return clone(data.sessions);
            case "save_sessions":
              data.sessions = clone(args.sessions || []);
              return null;
            case "get_sprint":
              return clone(data.sprint);
            case "save_sprint":
              data.sprint = clone(args.sprint);
              return null;
            case "get_tasks":
              return clone(data.tasks);
            case "save_tasks":
              data.tasks = clone(args.tasks || []);
              return null;
            case "get_presets":
              return clone(data.presets);
            case "save_presets":
              data.presets = clone(args.presets || []);
              return null;
            case "get_recent":
              return clone(data.recent);
            case "save_recent":
              data.recent = clone(args.items || []);
              return null;
            case "list_docs":
              return [...data.docs.entries()].map(([name, content]) =>
                fileEntry(name, content),
              );
            case "list_docs_tree":
              return clone(docsTree(data.docs));
            case "read_doc":
              return data.docs.get(args.filename) || "";
            case "write_doc":
              data.docs.set(args.filename, args.content || "");
              return null;
            case "delete_doc":
              data.docs.delete(args.filename);
              return null;
            case "create_doc_folder":
              data.docs.set(`${args.folderName}/.keep.md`, "");
              return null;
            case "rename_doc_folder": {
              const next = new Map<string, string>();
              for (const [name, content] of data.docs) {
                const renamed =
                  name === args.oldName || name.startsWith(`${args.oldName}/`)
                    ? `${args.newName}${name.slice(args.oldName.length)}`
                    : name;
                next.set(renamed, content);
              }
              data.docs = next;
              return null;
            }
            case "delete_doc_folder":
              for (const name of [...data.docs.keys()]) {
                if (
                  name === args.folderName ||
                  name.startsWith(`${args.folderName}/`)
                ) {
                  data.docs.delete(name);
                }
              }
              return null;
            case "move_doc": {
              const content = data.docs.get(args.filename);
              if (content !== undefined) {
                data.docs.delete(args.filename);
                data.docs.set(
                  `${args.targetFolder}/${args.filename.split("/").pop()}`,
                  content,
                );
              }
              return null;
            }
            case "rename_doc": {
              const content = data.docs.get(args.oldFilename);
              if (content !== undefined) {
                data.docs.delete(args.oldFilename);
                data.docs.set(args.newFilename, content);
              }
              return null;
            }
            case "list_notes":
              return [...data.notes.entries()].map(([name, content]) =>
                fileEntry(name, content),
              );
            case "read_note":
              return data.notes.get(args.filename) || "";
            case "write_note":
              data.notes.set(args.filename, args.content || "");
              return null;
            case "delete_note":
              data.notes.delete(args.filename);
              return null;
            case "rename_note": {
              const content = data.notes.get(args.oldFilename);
              if (content !== undefined) {
                data.notes.delete(args.oldFilename);
                data.notes.set(args.newFilename, content);
              }
              return null;
            }
            case "import_file": {
              const map = args.folder === "docs" ? data.docs : data.notes;
              map.set(args.filename, args.content || "");
              return null;
            }
            case "export_project":
              return JSON.stringify({ project: current, settings, data });
            case "reset_all_data_cmd":
              sessionStorage.setItem("openmesh.e2e.reset", "1");
              sessionStorage.removeItem("openmesh.e2e.settings");
              projects.clear();
              projectData.clear();
              currentProjectPath = null;
              return null;
            case "write_snapshot":
              data.notes.set(args.filename, args.content || "");
              return { success: true, filename: args.filename };
            case "agent_secret_status":
              return {
                configured: secretConfigured,
                store: "mock-secret-store",
              };
            case "agent_secret_set":
              if (scenario === "agent-secret-error")
                throw new Error("secret store unavailable");
              secretConfigured = true;
              return { configured: true, store: "mock-secret-store" };
            case "agent_secret_clear":
              if (scenario === "agent-secret-error")
                throw new Error("secret store unavailable");
              secretConfigured = false;
              return { configured: false, store: "mock-secret-store" };
            case "agent_provider_test":
              if (scenario === "agent-provider-error") {
                return {
                  ok: false,
                  model: args.request?.model || "sidecar-model",
                  baseUrl: args.request?.baseUrl || "",
                  latencyMs: 0,
                  error: "provider probe rejected the request",
                };
              }
              return {
                ok: true,
                model: args.request?.model || "sidecar-model",
                baseUrl: args.request?.baseUrl || "",
                latencyMs: 12,
                replyPreview: "mock provider reply",
              };
            case "scan_workspace_agent_sessions":
              if (scenario === "scan-error")
                return {
                  success: false,
                  sessions: [],
                  error: "permission denied",
                };
              return {
                success: true,
                sessions:
                  scenario === "mixed-sessions"
                    ? [
                        "codex",
                        "claude",
                        "opencode",
                        "cursor",
                        "gemini",
                        "grok",
                      ].map((tool, index) => ({
                        id: `scan-${tool}-session`,
                        toolName: tool,
                        title: `Scanned ${tool} session`,
                        sessionPath: `${path}/.${tool}/scanned.jsonl`,
                        fileName: `${tool}-scanned.jsonl`,
                        createdAt: now,
                        lastActiveAt: now,
                        fileSizeBytes: 128 + index,
                        summaryPreview: `Scanned ${tool} session preview`,
                        projectHint: current.name,
                        isReal: true,
                      }))
                    : [
                        {
                          id: "scan-session-1",
                          toolName: "codex",
                          title: "Scanned browser session",
                          sessionPath: `${path}/.codex/scanned.jsonl`,
                          fileName: "scanned.jsonl",
                          createdAt: now,
                          lastActiveAt: now,
                          fileSizeBytes: 128,
                          summaryPreview: "Scanned session preview",
                          projectHint: current.name,
                          isReal: true,
                        },
                      ],
              };
            case "read_foreign_session_transcript":
              if (scenario === "transcript-error")
                throw new Error("transcript unavailable");
              return {
                success: true,
                transcript: {
                  tool: "codex",
                  path: `${path}/.codex/scanned.jsonl`,
                  title: "Scanned browser session",
                  messages: [
                    { role: "user", text: "Imported browser session" },
                    { role: "assistant", text: "A prior reply" },
                  ],
                  truncated: false,
                  previewOnly: false,
                },
              };
            case "open_terminal":
            case "open_folder":
            case "open_agent_cli":
            case "run_command_preset":
              if (scenario === "command-error")
                return { success: false, error: "command launch failed" };
              if (scenario === "home-action-error")
                return { success: false, error: "home action failed" };
              return { success: true };
            case "pty_create":
              ptyCommands.push("pty_create");
              if (scenario === "pty-error") throw new Error("pty unavailable");
              return { id: args.id, shell: "zsh", cwd: args.cwd || path };
            case "pty_write":
            case "pty_resize":
            case "pty_kill":
            case "pty_kill_all":
              ptyCommands.push(command);
              return null;
            case "plugin:event|listen": {
              const event = String(args.event || "");
              const callbackId = Number(args.handler);
              const listeners = eventListeners.get(event) ?? new Set<number>();
              listeners.add(callbackId);
              eventListeners.set(event, listeners);
              return callbackId;
            }
            case "plugin:event|unlisten": {
              const event = String(args.event || "");
              const callbackId = Number(args.eventId);
              const listeners = eventListeners.get(event);
              listeners?.delete(callbackId);
              callbacks.delete(callbackId);
              return null;
            }
            case "agent_engine_cancel":
              return true;
            case "agent_workspace_tool": {
              const toolName = args.request?.toolName;
              if (scenario === "chat-tool-error" && toolName === "read_file") {
                throw new Error("fixture read failed");
              }
              if (toolName === "list_dir") {
                return JSON.stringify(["README.md", "src", "package.json"]);
              }
              if (toolName === "read_file") {
                return "# Browser project\n\nRead-only fixture file.";
              }
              if (toolName === "grep") {
                return "src/App.vue:1:Browser fixture match";
              }
              if (toolName === "git_diff") {
                return "No changes";
              }
              if (toolName === "create_handoff_draft") {
                return "Handoff draft created for browser coverage.";
              }
              if (toolName === "pending_questions") {
                return JSON.stringify({
                  openCount: 1,
                  items: [{ summary: "Review browser continuity" }],
                });
              }
              if (toolName === "link_session") {
                return "Linked foreign session to OpenMesh chat.";
              }
              return "Browser workspace tool result";
            }
            case "agent_engine_turn": {
              const question = String(args.request?.question || "");
              if (scenario === "chat-error") {
                throw new Error("provider request failed");
              }
              if (scenario === "chat-invalid-rich") {
                return {
                  assistantText: [
                    "# Rich fallback response",
                    "| Name | Value |\n| --- | --- |\n| Browser | fixture |",
                    "- first item\n- second item with [a link](https://example.test)",
                    "Inline `code` and a fenced block:",
                    "```canvas\nnot valid JSON\n```",
                    "```artifact\n{not valid JSON}\n```",
                  ].join("\n\n"),
                  toolSteps: [],
                  iterations: 1,
                  model: "sidecar-model",
                  provider: "OpenAiCompatible",
                  refused: false,
                  route: {
                    transport: "cli-proxy-api-sidecar",
                    providerLabel: "CLIProxyAPI sidecar",
                    endpointKind: "loopback-sidecar",
                    outcome: "completed",
                  },
                };
              }
              if (scenario === "chat-rich") {
                return {
                  assistantText: [
                    "# Rich browser response",
                    "Markdown **bold** and <img src=x onerror=alert(1)> sanitized.",
                    "```mermaid\ngraph TD\n  A[Start] --> B[End]\n```",
                    "```mermaid\nnot a valid mermaid diagram\n```",
                    '```canvas\n{"schema":"openmesh.canvas/1","id":"chat-auto-1","title":"Chat board","summary":"Saved from Chat","blocks":[{"type":"h1","text":"Chat board"},{"type":"stat","label":"Coverage","value":"All routes"},{"type":"callout","text":"Fixture callout","tone":"good"}]}\n```',
                    '```artifact\n{"kind":"raw","value":42}\n```',
                  ].join("\n\n"),
                  toolSteps: [],
                  iterations: 1,
                  model: "sidecar-model",
                  provider: "OpenAiCompatible",
                  refused: false,
                  route: {
                    transport: "cli-proxy-api-sidecar",
                    providerLabel: "CLIProxyAPI sidecar",
                    endpointKind: "loopback-sidecar",
                    outcome: "completed",
                  },
                };
              }
              if (scenario === "chat-progress") {
                const turnId = String(args.request?.turnId || "turn-browser");
                setTimeout(() => {
                  (
                    target.__OPENMESH_EMIT_EVENT__ as (
                      event: string,
                      payload: unknown,
                    ) => void
                  )("agent-turn-progress", {
                    kind: "tool_start",
                    turnId,
                    toolName: "git_status",
                    toolCallId: "git-status-1",
                  });
                }, 10);
                setTimeout(() => {
                  (
                    target.__OPENMESH_EMIT_EVENT__ as (
                      event: string,
                      payload: unknown,
                    ) => void
                  )("agent-turn-progress", {
                    kind: "tool_done",
                    turnId,
                    toolName: "git_status",
                    toolCallId: "git-status-1",
                    ok: true,
                    summary: "main\nNo changes",
                  });
                }, 35);
                await new Promise((resolve) => setTimeout(resolve, 80));
              }
              if (
                scenario === "slow-chat" ||
                question.includes("slow browser turn")
              ) {
                await new Promise((resolve) => setTimeout(resolve, 1200));
              }
              if (
                scenario === "patch" ||
                String(args.request?.question || "")
                  .toLowerCase()
                  .includes("patch")
              ) {
                return {
                  assistantText:
                    "Proposed patch patch-deadbeef is ready for review.",
                  toolSteps: [
                    {
                      toolName: "propose_patch",
                      toolCallId: "patch-call-1",
                      ok: true,
                      summary: "Proposed patch patch-deadbeef",
                    },
                  ],
                  iterations: 1,
                  model: "sidecar-model",
                  provider: "OpenAiCompatible",
                  refused: false,
                  route: {
                    transport: "cli-proxy-api-sidecar",
                    providerLabel: "CLIProxyAPI sidecar",
                    endpointKind: "loopback-sidecar",
                    outcome: "completed",
                  },
                };
              }
              return {
                assistantText: "mock sidecar reply",
                toolSteps: [],
                iterations: 1,
                model: "sidecar-model",
                provider: "OpenAiCompatible",
                refused: false,
                route: {
                  transport: "cli-proxy-api-sidecar",
                  providerLabel: "CLIProxyAPI sidecar",
                  endpointKind: "loopback-sidecar",
                  outcome: "completed",
                },
              };
            }
            case "agent_chat_load":
              return [];
            case "agent_chat_save":
              return null;
            case "agent_patch_get":
              return {
                id: args.patchId,
                status: patchStatus,
                summary: "Browser patch for end-to-end approval testing.",
                files: [
                  {
                    path: "src/browser-fixture.ts",
                    baseSha256: "base",
                    newContent: "export const browserFixture = true;",
                  },
                ],
                createdAt: now,
                appliedAt: patchStatus === "applied" ? now : null,
                rejectedAt: patchStatus === "rejected" ? now : null,
                rolledBackAt: patchStatus === "rolled_back" ? now : null,
                runId: "run-browser-patch",
              };
            case "agent_patch_apply":
              patchStatus = "applied";
              return {
                id: args.patchId,
                status: patchStatus,
                summary: "Browser patch for end-to-end approval testing.",
                files: [
                  {
                    path: "src/browser-fixture.ts",
                    baseSha256: "base",
                    newContent: "export const browserFixture = true;",
                  },
                ],
                createdAt: now,
                appliedAt: now,
                runId: "run-browser-patch",
              };
            case "agent_patch_reject":
              patchStatus = "rejected";
              return {
                id: args.patchId,
                status: patchStatus,
                summary: "Browser patch for end-to-end approval testing.",
                files: [
                  {
                    path: "src/browser-fixture.ts",
                    baseSha256: "base",
                    newContent: "export const browserFixture = true;",
                  },
                ],
                createdAt: now,
                rejectedAt: now,
                runId: "run-browser-patch",
              };
            case "agent_patch_rollback":
              patchStatus = "rolled_back";
              return {
                id: args.patchId,
                status: patchStatus,
                summary: "Browser patch for end-to-end approval testing.",
                files: [
                  {
                    path: "src/browser-fixture.ts",
                    baseSha256: "base",
                    newContent: "export const browserFixture = true;",
                  },
                ],
                createdAt: now,
                rolledBackAt: now,
                runId: "run-browser-patch",
              };
            case "agent_patch_summary":
              return "Patch patch-deadbeef: browser fixture change.";
            case "agent_recipe_list":
              return [
                {
                  id: "npm-typecheck",
                  title: "Typecheck",
                  argv: ["npm", "run", "typecheck"],
                  cwdRel: ".",
                  timeoutMs: 120000,
                },
              ];
            case "agent_recipe_suggest":
              return "npm-typecheck";
            case "agent_recipe_run":
              return {
                recipeId: args.request?.recipeId || "npm-typecheck",
                ok: true,
                exitCode: 0,
                timedOut: false,
                cancelled: false,
                stdout: "verified",
                stderr: "",
                durationMs: 12,
                runId: "run-browser-verify",
              };
            case "agent_recipe_cancel":
              return true;
            case "agent_delegate_brief":
              return `${path}/.openmesh/agent/delegate-browser.md`;
            case "agent_delegate_record_launch":
              return "delegate-run-browser";
            case "agent_handoff_approve":
              return `Approved handoff ${args.handoffId || "handoff-browser"}.`;
            case "context_health":
              if (scenario === "context-error") {
                throw new Error("context health unavailable");
              }
              if (scenario === "context-empty") {
                return {
                  path: `${path}/.openmesh/context.db`,
                  schema_version: 1,
                  sqlite_version: "3.45",
                  journal_mode: "wal",
                  document_count: 0,
                  fts_row_count: 0,
                  wal_mode_effective: true,
                  integrity_ok: true,
                };
              }
              if (scenario === "context-degraded") {
                return {
                  path: `${path}/.openmesh/context.db`,
                  schema_version: 1,
                  sqlite_version: "3.45",
                  journal_mode: "wal",
                  document_count: 3,
                  fts_row_count: 3,
                  wal_mode_effective: true,
                  integrity_ok: false,
                };
              }
              return {
                path: `${path}/.openmesh/context.db`,
                schema_version: 1,
                sqlite_version: "3.45",
                journal_mode: "wal",
                document_count: 3,
                fts_row_count: 3,
                wal_mode_effective: true,
                integrity_ok: true,
              };
            case "context_search": {
              const query = String(args.query || "").toLowerCase();
              if (query.includes("nonexistent")) return [];
              if (query.includes("secret")) {
                return [
                  clone({
                    ...contextResult,
                    document_id: "context-secret-1",
                    title: "Secret README.md",
                    snippet:
                      "Secret content is redacted in the browser inspector.",
                    sensitivity: "secret",
                  }),
                ];
              }
              if (scenario === "context-invalid-ref") {
                return [
                  clone({
                    ...contextResult,
                    canonical_ref: "not-a-canonical-ref",
                  }),
                ];
              }
              return [clone(contextResult)];
            }
            case "context_inspect":
              if (args.documentId === "context-secret-1") {
                return clone({
                  ...contextInspection,
                  document_id: "context-secret-1",
                  title: "Secret README.md",
                  sensitivity: "secret",
                  text: "Bearer do-not-return",
                });
              }
              return clone(contextInspection);
            case "context_refresh":
              if (scenario === "context-error") {
                throw new Error("context refresh unavailable");
              }
              if (scenario === "context-partial") {
                return {
                  project_id: current.id,
                  status: "PARTIAL",
                  started_at: now,
                  completed_at: now,
                  discovered: 3,
                  indexed: 1,
                  updated: 1,
                  unchanged: 0,
                  removed: 0,
                  skipped: 1,
                  failed: 1,
                  receipts: [],
                };
              }
              if (scenario === "context-empty") {
                return {
                  project_id: current.id,
                  status: "COMPLETE",
                  started_at: now,
                  completed_at: now,
                  discovered: 0,
                  indexed: 0,
                  updated: 0,
                  unchanged: 0,
                  removed: 0,
                  skipped: 0,
                  failed: 0,
                  receipts: [],
                };
              }
              return {
                project_id: current.id,
                status: "COMPLETE",
                started_at: now,
                completed_at: now,
                discovered: 3,
                indexed: 1,
                updated: 1,
                unchanged: 1,
                removed: 0,
                skipped: 0,
                failed: 0,
                receipts: [],
              };
            case "canvas_list":
              return clone(data.canvases);
            case "canvas_create": {
              const doc = {
                id: nextId("canvas"),
                title: args.title || "Agent network",
                schemaVersion: "1",
                nodes: [],
                edges: [],
                updatedAt: Date.now(),
              };
              data.canvases.push(doc);
              return clone(doc);
            }
            case "canvas_load":
              return clone(
                data.canvases.find((item: any) => item.id === args.id) ||
                  data.canvases[0],
              );
            case "canvas_add_node": {
              const canvas =
                data.canvases.find((item: any) => item.id === args.canvasId) ||
                data.canvases[0];
              canvas.nodes.push({
                id: nextId("node"),
                label: args.label,
                kind: args.kind || "machine",
                x: canvas.nodes.length * 180 + 80,
                y: 80,
              });
              canvas.updatedAt = Date.now();
              return clone(canvas);
            }
            case "canvas_connect": {
              const canvas =
                data.canvases.find((item: any) => item.id === args.canvasId) ||
                data.canvases[0];
              canvas.edges.push({
                id: nextId("edge"),
                from: args.from,
                to: args.to,
              });
              canvas.updatedAt = Date.now();
              return clone(canvas);
            }
            case "canvas_delete_node": {
              const canvas =
                data.canvases.find((item: any) => item.id === args.canvasId) ||
                data.canvases[0];
              canvas.nodes = canvas.nodes.filter(
                (item: any) => item.id !== args.nodeId,
              );
              canvas.edges = canvas.edges.filter(
                (edge: any) =>
                  edge.from !== args.nodeId && edge.to !== args.nodeId,
              );
              return clone(canvas);
            }
            case "canvas_auto_ui_list":
              return clone(data.autoUi);
            case "canvas_auto_ui_load":
              return clone(
                data.autoUi.find((item: any) => item.id === args.id) ||
                  data.autoUi[0],
              );
            case "canvas_auto_ui_delete":
              data.autoUi = data.autoUi.filter(
                (item: any) => item.id !== args.id,
              );
              return null;
            case "canvas_auto_ui_upsert": {
              const doc = clone(args.document);
              const index = data.autoUi.findIndex(
                (item: any) => item.id === doc.id,
              );
              if (index >= 0) data.autoUi[index] = doc;
              else data.autoUi.push(doc);
              return clone(doc);
            }
            case "canvas_board_list":
              return clone(data.boards);
            case "canvas_board_create": {
              const board = {
                schema: "openmesh.board/1",
                id: nextId("board"),
                title: args.title || "Board",
                engine: "excalidraw",
                scene: { elements: [], appState: {}, files: {} },
                updatedAt: Date.now(),
              };
              data.boards.push(board);
              return clone(board);
            }
            case "canvas_board_load":
              return clone(
                data.boards.find((item: any) => item.id === args.id) ||
                  data.boards[0],
              );
            case "canvas_board_upsert": {
              const board = clone(args.document);
              const index = data.boards.findIndex(
                (item: any) => item.id === board.id,
              );
              if (index >= 0) data.boards[index] = board;
              else data.boards.push(board);
              return clone(board);
            }
            case "canvas_board_save_scene": {
              boardSceneSaves.push(clone(args.scene));
              const board =
                data.boards.find((item: any) => item.id === args.id) ||
                data.boards[0];
              board.scene = clone(args.scene);
              board.updatedAt = Date.now();
              return clone(board);
            }
            case "canvas_board_delete":
              data.boards = data.boards.filter(
                (item: any) => item.id !== args.id,
              );
              return null;
            case "canvas_board_add_sticky": {
              const board =
                data.boards.find(
                  (item: any) =>
                    item.id === (args.boardId || data.boards[0]?.id),
                ) || data.boards[0];
              if (board) {
                board.scene.elements = [
                  ...(board.scene.elements || []),
                  { id: nextId("sticky"), type: "text", text: args.text },
                ];
                board.updatedAt = Date.now();
              }
              return clone(board);
            }
            case "canvas_board_connect":
              return clone(
                data.boards.find(
                  (item: any) =>
                    item.id === (args.boardId || data.boards[0]?.id),
                ) || data.boards[0],
              );
            case "continuity_hub_summary":
              return continuityView(path).summary;
            case "continuity_pending":
              if (scenario === "continuity-error") throw new Error("continuity unavailable");
              return continuityView(path).pending;
            case "continuity_digest":
              return continuityView(path).digest;
            case "mesh_list_peers":
              return clone(data.continuity.peers);
            case "mesh_list_envelopes":
              return [
                {
                  envelopeId: "envelope-1",
                  mailbox: "inbox",
                  fromPeer: { label: "Browser peer" },
                  generatedAt: now,
                  evidenceItemCount: 1,
                  handoffIdCount: 0,
                  limitationCount: 0,
                  attributedTo: "fixture",
                },
              ];
            case "mesh_add_peer": {
              const request = args.request || {};
              const peer = {
                protocolVersion: "1",
                peerId: request.peerId || nextId("peer"),
                label: request.label,
                notes: request.notes,
                lanAddress: request.lanAddress,
                createdAt: now,
                updatedAt: now,
              };
              data.continuity.peers.push(peer);
              return clone(peer);
            }
            case "mesh_query_peer":
              return {
                protocolVersion: "1",
                queryId: nextId("query"),
                peerId: args.request?.peer || "peer-browser",
                peerLabel: "Browser peer",
                question: args.request?.question || "",
                answerText: "Offline peer answer",
                generatedAt: now,
                readOnly: true,
                freshness: {
                  statement: "Fresh fixture",
                  evaluatedAt: now,
                  tier: args.request?.tier || "low-impact",
                  isSufficient: true,
                  confidenceLabel: "high",
                  oldestEvidenceAgeSeconds: 1,
                  staleWarnings: [],
                  evidenceSourceIds: [],
                },
                refused: false,
                envelopeIds: ["envelope-1"],
                evidenceSummaries: ["Fixture evidence"],
                limitations: [],
              };
            case "relay_list_audit":
              return [];
            case "online_proxy_status":
              return clone(data.continuity.onlineProxy);
            case "online_proxy_init":
              data.continuity.onlineProxy = {
                protocolVersion: "1",
                proxyId: "proxy-browser",
                workspaceId: "workspace-browser",
                ownerLabel: args.request?.ownerLabel || current.name,
                mode: args.request?.mode || "local-scaffold",
                defaultFreshnessTier: "standard",
                useRelayReceived: true,
                createdAt: now,
                updatedAt: now,
              };
              return clone(data.continuity.onlineProxy);
            case "online_proxy_ask":
              return onlineAnswer(args.request?.question || "");
            case "team_workspace_status":
              return clone(data.continuity.team);
            case "team_init":
              data.continuity.team = {
                protocolVersion: "1",
                teamId: args.request?.teamId || "team-browser",
                displayName: args.request?.name || "Browser team",
                hostWorkspaceId: "workspace-browser",
                members: [
                  {
                    memberId: "member-owner",
                    label: args.request?.ownerLabel || current.name,
                    role: "owner",
                    joinedAt: now,
                  },
                ],
                createdAt: now,
                updatedAt: now,
                limitations: [],
              };
              return clone(data.continuity.team);
            case "team_add_member":
              if (!data.continuity.team) throw new Error("team required");
              data.continuity.team.members.push({
                memberId: nextId("member"),
                label: args.request?.label || "Member",
                role: args.request?.role || "member",
                meshPeerId: args.request?.meshPeerId,
                joinedAt: now,
              });
              return clone(data.continuity.team);
            case "team_remove_member":
              if (data.continuity.team) {
                data.continuity.team.members =
                  data.continuity.team.members.filter(
                    (member: any) => member.memberId !== args.request?.memberId,
                  );
              }
              return clone(data.continuity.team);
            case "team_trust_policy_status":
              return data.continuity.trust
                ? clone(data.continuity.trust)
                : null;
            case "team_trust_init":
              data.continuity.trust = trustView(path);
              return clone(data.continuity.trust);
            case "team_trust_set_remote_query":
              data.continuity.trust = {
                ...trustView(path),
                remoteQueryEnabled: !!args.request?.enabled,
              };
              return clone(data.continuity.trust);
            case "team_trust_set_query_mode":
              data.continuity.trust = {
                ...trustView(path),
                queryAllowlistMode: args.request?.mode || "deny-all",
              };
              return clone(data.continuity.trust);
            case "team_trust_allowlist_add":
              data.continuity.trust = {
                ...trustView(path),
                queryAllowlist: [
                  ...trustView(path).queryAllowlist,
                  {
                    meshPeerId: args.request?.meshPeerId,
                    memberId: args.request?.memberId,
                    note: args.request?.note,
                    addedAt: now,
                  },
                ],
              };
              return clone(data.continuity.trust);
            case "team_trust_allowlist_remove":
              data.continuity.trust = {
                ...trustView(path),
                queryAllowlist: trustView(path).queryAllowlist.filter(
                  (item: any) =>
                    item.meshPeerId !== args.request?.meshPeerId &&
                    item.memberId !== args.request?.memberId,
                ),
              };
              return clone(data.continuity.trust);
            case "team_trust_audit_list":
              return [];
            case "connector_list":
              return [
                {
                  protocolVersion: "1",
                  connectorId: "local-browser",
                  kind: "local",
                  displayName: "Browser fixture",
                  role: "read-only",
                  enabled: true,
                  limitations: [],
                },
              ];
            case "org_graph_show":
              return {
                protocolVersion: "1",
                teamId: data.continuity.team?.teamId || "team-browser",
                generatedAt: now,
                nodes: [
                  {
                    id: "workspace-browser",
                    kind: "workspace",
                    label: current.name,
                    evidence: "fixture",
                  },
                ],
                edges: [],
                limitations: [],
              };
            case "pilot_status":
              return {
                protocolVersion: "1",
                workspaceId: "workspace-browser",
                generatedAt: now,
                pilotReady: true,
                passCount: 2,
                warnCount: 0,
                failCount: 0,
                checks: [],
                threatNotes: [],
                runbook: [],
                limitations: [],
              };
            case "rc_status":
              return {
                protocolVersion: "1",
                workspaceId: "workspace-browser",
                generatedAt: now,
                rcReady: true,
                p0FailCount: 0,
                p1FailCount: 0,
                openCount: 0,
                checks: [],
                regressionMatrix: [],
                freezePolicy: {
                  featuresFrozen: false,
                  allowed: [],
                  forbidden: [],
                  summary: "Fixture",
                },
                limitations: [],
              };
            case "lan_serve_status":
              return {
                running: data.continuity.lanRunning,
                protocol: "1",
                projectPath: path,
                peerId: "peer-local",
                ownerLabel: current.name,
                projectId: current.id,
                httpHost: "127.0.0.1",
                httpPort: 41778,
                udpPort: 41779,
                startedAt: data.continuity.lanRunning ? now : undefined,
                note: "Browser fixture listener",
              };
            case "lan_serve_start":
              data.continuity.lanRunning = true;
              return {
                running: true,
                protocol: "1",
                projectPath: path,
                peerId: "peer-local",
                ownerLabel: args.request?.ownerLabel || current.name,
                projectId: current.id,
                httpHost: "127.0.0.1",
                httpPort: args.request?.httpPort || 41778,
                udpPort: args.request?.udpPort || 41779,
                startedAt: now,
                note: "Browser fixture listener",
              };
            case "lan_serve_stop":
              data.continuity.lanRunning = false;
              return { running: false, protocol: "1", note: "Stopped" };
            case "lan_discover":
              return [
                {
                  protocol: "1",
                  projectId: "remote-project",
                  ownerLabel: "Browser peer",
                  peerId: "peer-browser",
                  host: "127.0.0.1",
                  httpPort: 41778,
                  startedAt: now,
                  lastSeenAt: now,
                  address: "127.0.0.1:41778",
                },
              ];
            case "lan_list_last_peers":
              return [];
            case "lan_list_approved_packages":
              return ["package-browser-1"];
            case "lan_send_package":
              return { ok: true };
            case "lan_ask_peer":
              return {
                protocolVersion: "1",
                queryId: nextId("query"),
                peerId: "peer-browser",
                peerLabel: "Browser peer",
                question: args.request?.question || "",
                answerText: "Live fixture answer",
                generatedAt: now,
                readOnly: true,
                freshness: {
                  statement: "Fresh fixture",
                  evaluatedAt: now,
                  tier: args.request?.tier || "low-impact",
                  isSufficient: true,
                  confidenceLabel: "high",
                  oldestEvidenceAgeSeconds: 1,
                  staleWarnings: [],
                  evidenceSourceIds: [],
                },
                refused: false,
                envelopeIds: [],
                evidenceSummaries: ["Fixture evidence"],
                limitations: [],
              };
            case "lan_probe_presence":
              return (args.request?.targets || []).map((item: any) =>
                lanPresence(item.address),
              );
            case "lan_probe_address":
              return lanPresence(args.address);
            case "lan_chat_send": {
              const message = {
                message: {
                  protocol: "1",
                  messageId: nextId("message"),
                  fromPeerId: "peer-local",
                  fromLabel: args.request?.fromLabel || current.name,
                  text: args.request?.text || "",
                  sentAt: now,
                },
                direction: "outbound",
                peerKey: args.request?.to || "",
                storedAt: now,
              };
              data.continuity.chat.push(message);
              return clone(message);
            }
            case "lan_chat_list":
              return clone(data.continuity.chat);
            case "extensions_list":
              return clone(extensionInventory);
            case "extensions_catalog":
              return clone(extensionCatalog);
            case "extensions_set_enabled": {
              const request = args.request || {};
              const collection =
                request.kind === "skill"
                  ? extensionInventory.skills
                  : request.kind === "hook"
                    ? extensionInventory.hooks
                    : extensionInventory.plugins;
              const item = collection.find(
                (entry: any) => entry.id === request.id,
              );
              if (item) item.enabled = !!request.enabled;
              settings.extensions[`${request.kind}s`][request.id] =
                !!request.enabled;
              return clone(settings.extensions);
            }
            case "extensions_install":
              extensionInventory.skills.push({
                id: "browser-installed-skill",
                name: "Browser Installed Skill",
                description: "Installed from a browser fixture folder",
                body: "Installed skill body",
                enabled: true,
                source: "project",
                pluginId: null,
                path: args.sourcePath || "/tmp/browser-installed/SKILL.md",
              });
              return {
                installed: "browser-installed-skill",
                path: args.sourcePath || "/tmp/browser-installed",
              };
            case "voice_model_catalog":
              return [
                {
                  id: "tiny-stt",
                  name: "Tiny STT",
                  sizeBytes: 1024,
                  license: "MIT",
                  languages: ["en"],
                },
              ];
            case "voice_model_status":
              return "Cloud STT available";
            case "plugin:dialog|open":
              return "/tmp/browser-extension-folder";
            case "download_and_open_update":
              if (scenario === "update-open-failure") {
                return {
                  path: "/tmp/OpenMesh-browser-installer.dmg",
                  opened: false,
                  nextSteps: "Open the downloaded installer manually.",
                  platformOs: "macos",
                };
              }
              return {
                path: "/tmp/OpenMesh-browser-installer.dmg",
                opened: true,
                nextSteps: "Browser fixture installer opened.",
                platformOs: "macos",
              };
            case "cliproxy_provider_list":
              if (scenario === "provider-error") {
                throw new Error("sidecar provider inventory unavailable");
              }
              return clone(providerCatalog);
            case "cliproxy_provider_save": {
              if (scenario === "provider-mutation-error")
                throw new Error("sidecar provider write rejected");
              const request = args.request || {};
              const section = request.section;
              const providers = providerCatalog.providers.filter(
                (p: any) => p.section === section,
              );
              const existing =
                request.index === undefined
                  ? null
                  : providerCatalog.providers.find(
                      (p: any) =>
                        p.section === section && p.index === request.index,
                    );
              const record = {
                section,
                sectionLabel:
                  section === "codex-api-key"
                    ? "Codex API"
                    : section === "openai-compatibility"
                      ? "OpenAI compatible"
                      : section === "claude-api-key"
                        ? "Claude API"
                        : "Gemini API",
                index: existing?.index ?? providers.length,
                name: request.name || existing?.name || `${section}-browser`,
                baseUrl:
                  request.baseUrl ??
                  existing?.baseUrl ??
                  (existing?.endpointConfigured
                    ? "https://existing.example.com/v1"
                    : null),
                endpointConfigured: Boolean(
                  request.baseUrl || existing?.endpointConfigured,
                ),
                models: request.models || existing?.models || [],
                priority:
                  request.priority === undefined
                    ? (existing?.priority ?? null)
                    : request.priority,
                enabled:
                  request.enabled === undefined
                    ? (existing?.enabled ?? true)
                    : request.enabled,
                keyConfigured: Boolean(
                  request.apiKey || existing?.keyConfigured,
                ),
                keyCount: request.apiKey || existing?.keyConfigured ? 1 : 0,
                headerCount: existing?.headerCount ?? 0,
              };
              providerCatalog.providers = existing
                ? providerCatalog.providers.map((p: any) =>
                    p.section === section && p.index === request.index
                      ? record
                      : p,
                  )
                : [...providerCatalog.providers, record];
              refreshProviderIndexes();
              return clone(providerCatalog);
            }
            case "cliproxy_provider_toggle": {
              if (scenario === "provider-mutation-error")
                throw new Error("sidecar provider toggle rejected");
              const request = args.request || {};
              providerCatalog.providers = providerCatalog.providers.map(
                (p: any) =>
                  p.section === request.section && p.index === request.index
                    ? { ...p, enabled: !!request.enabled }
                    : p,
              );
              refreshProviderIndexes();
              return clone(providerCatalog);
            }
            case "cliproxy_provider_reorder": {
              if (scenario === "provider-mutation-error")
                throw new Error("sidecar provider reorder rejected");
              const request = args.request || {};
              const sectionProviders = providerCatalog.providers.filter(
                (p: any) => p.section === request.section,
              );
              const [moved] = sectionProviders.splice(request.fromIndex, 1);
              if (moved) sectionProviders.splice(request.toIndex, 0, moved);
              const next = providerCatalog.providers.filter(
                (p: any) => p.section !== request.section,
              );
              providerCatalog.providers = [...next, ...sectionProviders];
              refreshProviderIndexes();
              return clone(providerCatalog);
            }
            case "cliproxy_provider_delete": {
              if (scenario === "provider-mutation-error")
                throw new Error("sidecar provider delete rejected");
              const request = args.request || {};
              providerCatalog.providers = providerCatalog.providers.filter(
                (p: any) =>
                  !(p.section === request.section && p.index === request.index),
              );
              refreshProviderIndexes();
              return clone(providerCatalog);
            }
            case "cliproxy_routing_strategy":
              if (scenario === "provider-mutation-error")
                throw new Error("sidecar routing write rejected");
              providerCatalog.routingStrategy =
                args.request?.strategy || "round-robin";
              providerCatalog.revision = nextRevision();
              return clone(providerCatalog);
            case "oauth_runtime_status": {
              if (scenario === "runtime-error") {
                throw new Error("runtime status unavailable");
              }
              if (scenario === "runtime-unauthorized") {
                return clone({
                  ...runtimeStatus,
                  managementStatus: "unauthorized",
                  managementError: "management secret rejected by sidecar",
                  dataPlaneStatus: "notConfigured",
                  configurationStatus: "unavailable",
                  configuration: null,
                  providers: [],
                });
              }
              if (scenario === "runtime-chat-unauthorized") {
                return clone({
                  ...runtimeStatus,
                  dataPlaneStatus: "unauthorized",
                  dataPlaneError: "chat key rejected by sidecar",
                });
              }
              if (scenario === "runtime-unsupported") {
                return clone({
                  ...runtimeStatus,
                  configurationStatus: "unsupported",
                  configurationError: "configuration endpoint not supported",
                  configuration: null,
                  providers: [],
                });
              }
              if (scenario === "runtime-empty") {
                return clone({ ...runtimeStatus, providers: [] });
              }
              if (scenario === "runtime-unavailable") {
                return clone({
                  ...runtimeStatus,
                  managementStatus: "unreachable",
                  managementSecretConfigured: true,
                  managementError: "loopback sidecar unavailable",
                  dataPlaneStatus: "notConfigured",
                  sidecarClientKeyConfigured: false,
                  configurationStatus: "unavailable",
                  configuration: null,
                });
              }
              return clone({
                ...runtimeStatus,
                managementPort: oauthManagementPort,
                managementEndpoint: `http://127.0.0.1:${oauthManagementPort}/v0/management/`,
                dataPlaneEndpoint: `http://127.0.0.1:${oauthManagementPort}/v1`,
                managementStatus: oauthSecretConfigured
                  ? "ready"
                  : "notConfigured",
                managementSecretConfigured: oauthSecretConfigured,
                dataPlaneStatus:
                  oauthSidecarEnabled && oauthSidecarClientKeyConfigured
                    ? "ready"
                    : "notConfigured",
                sidecarClientKeyConfigured: oauthSidecarClientKeyConfigured,
                configurationStatus: oauthSecretConfigured
                  ? "available"
                  : "not-configured",
                configuration: oauthSecretConfigured
                  ? {
                      ...runtimeStatus.configuration,
                      port: oauthManagementPort,
                    }
                  : null,
              });
            }
            case "oauth_connection_status": {
              const dataPlaneStatus =
                oauthSidecarEnabled && oauthSidecarClientKeyConfigured
                  ? "ready"
                  : "notConfigured";
              return {
                managementPort: oauthManagementPort,
                endpoint: `http://127.0.0.1:${oauthManagementPort}/v0/management`,
                secretConfigured: oauthSecretConfigured,
                sidecarClientKeyConfigured: oauthSidecarClientKeyConfigured,
                dataPlaneStatus,
                dataPlaneEndpoint: `http://127.0.0.1:${oauthManagementPort}/v1`,
                dataPlaneError: null,
                sidecarEnabled: oauthSidecarEnabled,
                status: oauthSecretConfigured ? "ready" : "notConfigured",
                error: null,
                providers: [],
              };
            }
            case "oauth_admin_list":
              return {
                revision: oauthAdminRevision,
                files: scenario === "oauth-admin-empty" ? [] : clone(oauthAuthFiles),
              };
            case "oauth_admin_exclusions":
              return {
                revision: oauthAdminRevision,
                exclusions: {
                  provider: args.provider || "codex",
                  models: ["gpt-image-*"],
                },
              };
            case "oauth_model_definitions":
              if (scenario === "oauth-model-empty") return [];
              return [
                { id: "sidecar-model", displayName: "Sidecar model" },
                { id: "gpt-image-1.5" },
              ];
            case "oauth_admin_save_exclusions":
              oauthAdminRevision = "r1-fedcba9876543210";
              return {
                revision: oauthAdminRevision,
                exclusions: {
                  provider: args.request?.provider || "codex",
                  models: args.request?.models || ["gpt-image-1.5"],
                },
              };
            case "oauth_admin_set_status": {
              const request = args.request || {};
              oauthAuthFiles = oauthAuthFiles.map((file: any) =>
                file.name === request.name
                  ? { ...file, enabled: !request.disabled }
                  : file,
              );
              oauthAdminRevision = "r1-fedcba9876543210";
              return {
                revision: oauthAdminRevision,
                files: clone(oauthAuthFiles),
              };
            }
            case "oauth_admin_set_priority": {
              const request = args.request || {};
              oauthAuthFiles = oauthAuthFiles.map((file: any) =>
                file.name === request.name
                  ? { ...file, priority: request.priority }
                  : file,
              );
              oauthAdminRevision = "r1-fedcba9876543210";
              return {
                revision: oauthAdminRevision,
                files: clone(oauthAuthFiles),
              };
            }
            case "oauth_set_management_port":
              oauthManagementPort = Number(args.port) || oauthManagementPort;
              return {
                managementPort: oauthManagementPort,
                endpoint: `http://127.0.0.1:${oauthManagementPort}/v0/management`,
                dataPlaneEndpoint: `http://127.0.0.1:${oauthManagementPort}/v1`,
                sidecarEnabled: oauthSidecarEnabled,
                secretConfigured: oauthSecretConfigured,
                sidecarClientKeyConfigured: oauthSidecarClientKeyConfigured,
              };
            case "oauth_set_sidecar_enabled":
              oauthSidecarEnabled = !!args.enabled;
              return {
                managementPort: oauthManagementPort,
                endpoint: `http://127.0.0.1:${oauthManagementPort}/v0/management`,
                dataPlaneEndpoint: `http://127.0.0.1:${oauthManagementPort}/v1`,
                sidecarEnabled: oauthSidecarEnabled,
                secretConfigured: oauthSecretConfigured,
                sidecarClientKeyConfigured: oauthSidecarClientKeyConfigured,
              };
            case "oauth_set_sidecar_client_key":
              oauthSidecarClientKeyConfigured = true;
              return {
                managementPort: oauthManagementPort,
                endpoint: `http://127.0.0.1:${oauthManagementPort}/v0/management`,
                dataPlaneEndpoint: `http://127.0.0.1:${oauthManagementPort}/v1`,
                sidecarEnabled: oauthSidecarEnabled,
                secretConfigured: oauthSecretConfigured,
                sidecarClientKeyConfigured: true,
              };
            case "oauth_clear_sidecar_client_key":
              oauthSidecarClientKeyConfigured = false;
              return {
                managementPort: oauthManagementPort,
                endpoint: `http://127.0.0.1:${oauthManagementPort}/v0/management`,
                dataPlaneEndpoint: `http://127.0.0.1:${oauthManagementPort}/v1`,
                sidecarEnabled: oauthSidecarEnabled,
                secretConfigured: oauthSecretConfigured,
                sidecarClientKeyConfigured: false,
              };
            case "oauth_set_management_secret":
              oauthSecretConfigured = true;
              return {
                managementPort: oauthManagementPort,
                endpoint: `http://127.0.0.1:${oauthManagementPort}/v0/management`,
                dataPlaneEndpoint: `http://127.0.0.1:${oauthManagementPort}/v1`,
                sidecarEnabled: oauthSidecarEnabled,
                secretConfigured: true,
                sidecarClientKeyConfigured: oauthSidecarClientKeyConfigured,
              };
            case "oauth_clear_management_secret":
              oauthSecretConfigured = false;
              return {
                managementPort: oauthManagementPort,
                endpoint: `http://127.0.0.1:${oauthManagementPort}/v0/management`,
                dataPlaneEndpoint: `http://127.0.0.1:${oauthManagementPort}/v1`,
                sidecarEnabled: oauthSidecarEnabled,
                secretConfigured: false,
                sidecarClientKeyConfigured: oauthSidecarClientKeyConfigured,
              };
            case "oauth_start":
              oauthPollCount = 0;
              return {
                url: "https://oauth.example.test/authorize",
                state: "browser-oauth-state",
              };
            case "oauth_status":
              oauthPollCount += 1;
              if (scenario === "oauth-error") {
                return {
                  status: "error",
                  error: "provider authorization failed",
                };
              }
              return {
                status:
                  scenario === "oauth-success" || oauthPollCount > 1
                    ? "ok"
                    : "pending",
                error: null,
              };
            case "oauth_submit_callback":
              oauthPollCount = 1;
              return null;
            case "oauth_cancel":
            case "oauth_open_url":
              return null;
            default:
              return null;
          }
        },
      };
    },
    {
      scenario: options.scenario ?? "seeded",
      runtime: options.runtime ?? "tauri",
    },
  );
}
