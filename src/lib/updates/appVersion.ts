import packageJson from "../../../package.json";

/** Running app version from package.json (kept in sync with tauri.conf / Cargo.toml). */
export function getAppVersion(): string {
  return String(packageJson.version ?? "0.0.0");
}
