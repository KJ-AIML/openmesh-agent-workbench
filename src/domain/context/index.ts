// ============================================================================
// OpenMesh Context Domain — Public Barrel
// ============================================================================
//
// Dev Track 0.1.2.2 — ContextSource Domain Model
//
// This module establishes the versioned common domain contract for work-context
// sources. It is storage-agnostic: no SQLite, no Tauri, no FS I/O.
//
// Specification conflict resolved:
//   Dev Track 0.1.2.2 lists Docs/Notes/Snapshots/Tasks/Recent/Sessions.
//   Canonical Blueprint 17.1 lists doc/note/snapshot/task/work-event/
//   agent-session/git/connector (no "recent").
//   Resolution: "recent" is added as a TRANSITIONAL kind to bridge current
//   RecentItem data until OpenMesh 0.1.3 introduces WorkEvent.
//   Reserved kinds (work-event, git, connector) are defined but have no
//   mappers yet.
//
// Module map:
//   types.ts           — enums, interfaces, constants
//   canonicalRef.ts    — pure canonicalRef + sourceId derivation
//   validators.ts      — pure runtime validators
//   mappers.ts         — pure current-source mappers
//   documentBuilder.ts — pure ContextDocument factory
// ============================================================================

export * from "./types";
export * from "./canonicalRef";
export * from "./validators";
export * from "./mappers";
export * from "./documentBuilder";
