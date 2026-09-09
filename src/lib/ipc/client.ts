/**
 * Canonical Tauri invoke transport. Feature code must not import `invoke`
 * from `@tauri-apps/api/core`.
 */
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import {
  isTypedCommandName,
  type TypedCommandName,
} from "./catalog";

export type IpcTransport = {
  invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
};

const defaultTransport: IpcTransport = {
  invoke: (command, args) =>
    args === undefined ? tauriInvoke(command) : tauriInvoke(command, args),
};

let transport: IpcTransport = defaultTransport;

export function setIpcTransport(next: IpcTransport | null): void {
  transport = next ?? defaultTransport;
}

export function getIpcTransport(): IpcTransport {
  return transport;
}

export class IpcError extends Error {
  readonly code: string;
  readonly details?: unknown;

  constructor(code: string, message: string, details?: unknown) {
    super(message);
    this.name = "IpcError";
    this.code = code;
    this.details = details;
  }
}

function classifyCode(message: string): string {
  const lower = message.toLowerCase();
  if (
    lower.includes("api key") ||
    lower.includes("not configured") ||
    lower.includes("missing credential")
  ) {
    return "missing_credential";
  }
  if (
    lower.includes("forbidden") ||
    lower.includes("unauthorized") ||
    lower.includes("not registered") ||
    lower.includes("not allowed")
  ) {
    return "forbidden";
  }
  if (
    lower.includes("invalid path") ||
    lower.includes("escapes") ||
    lower.includes("project not")
  ) {
    return "invalid_path";
  }
  if (lower.includes("cancel")) return "cancelled";
  if (lower.includes("not found")) return "not_found";
  if (lower.includes("conflict") || lower.includes("already")) return "conflict";
  return "ipc_failure";
}

export function normalizeIpcError(err: unknown): IpcError {
  if (err instanceof IpcError) return err;
  const message = err instanceof Error ? err.message : String(err);
  return new IpcError(classifyCode(message), message);
}

export async function invokeRaw(
  command: string,
  args?: Record<string, unknown>,
): Promise<unknown> {
  try {
    return args === undefined
      ? await transport.invoke(command)
      : await transport.invoke(command, args);
  } catch (err) {
    throw normalizeIpcError(err);
  }
}

export async function invokeTyped<TResult>(
  command: TypedCommandName,
  args?: Record<string, unknown>,
): Promise<TResult> {
  if (!isTypedCommandName(command)) {
    throw new IpcError("unknown_command", `Unknown typed IPC command: ${command}`);
  }
  return invokeRaw(command, args) as Promise<TResult>;
}
