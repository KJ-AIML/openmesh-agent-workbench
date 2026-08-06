/**
 * In-app PTY adapter for the Chat Terminal panel.
 * Tauri: portable-pty sessions. Web: mocked failure (desktop-only).
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getRuntimeKind } from "./environment";

export type PtyCreateResult = {
  id: string;
  shell: string;
  cwd: string;
};

export type PtyDataEvent = {
  id: string;
  data: string;
};

export type PtyExitEvent = {
  id: string;
};

export async function createPty(opts: {
  id: string;
  cwd: string;
  cols?: number;
  rows?: number;
}): Promise<PtyCreateResult> {
  const runtime = getRuntimeKind();
  if (runtime !== "tauri") {
    throw new Error("Embedded terminal requires the desktop app.");
  }
  return invoke<PtyCreateResult>("pty_create", {
    id: opts.id,
    cwd: opts.cwd,
    cols: opts.cols ?? null,
    rows: opts.rows ?? null,
  });
}

export async function writePty(id: string, data: string): Promise<void> {
  if (getRuntimeKind() !== "tauri") return;
  await invoke("pty_write", { id, data });
}

export async function resizePty(
  id: string,
  cols: number,
  rows: number,
): Promise<void> {
  if (getRuntimeKind() !== "tauri") return;
  await invoke("pty_resize", { id, cols, rows });
}

export async function killPty(id: string): Promise<void> {
  if (getRuntimeKind() !== "tauri") return;
  await invoke("pty_kill", { id });
}

export async function killAllPtys(): Promise<void> {
  if (getRuntimeKind() !== "tauri") return;
  await invoke("pty_kill_all");
}

export async function listenPtyData(
  handler: (ev: PtyDataEvent) => void,
): Promise<UnlistenFn> {
  return listen<PtyDataEvent>("pty-data", (event) => {
    if (event.payload) handler(event.payload);
  });
}

export async function listenPtyExit(
  handler: (ev: PtyExitEvent) => void,
): Promise<UnlistenFn> {
  return listen<PtyExitEvent>("pty-exit", (event) => {
    if (event.payload) handler(event.payload);
  });
}
