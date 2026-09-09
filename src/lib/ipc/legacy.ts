import { isTypedCommandName } from "./catalog";
import { invokeRaw, IpcError } from "./client";

/**
 * Explicit compatibility path for unmigrated registered commands.
 * Must not be used for names in the typed catalog.
 */
export async function legacyInvoke<TResult>(
  command: string,
  args?: Record<string, unknown>,
): Promise<TResult> {
  if (isTypedCommandName(command)) {
    throw new IpcError(
      "contract_conflict",
      `Command "${command}" is on the typed IPC catalog; do not use legacyInvoke`,
    );
  }
  return invokeRaw(command, args) as Promise<TResult>;
}
