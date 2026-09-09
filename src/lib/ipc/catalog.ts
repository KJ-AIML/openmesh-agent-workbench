/**
 * Migrated Tauri command identities (A8).
 * CI parses TYPED_COMMAND_NAMES. Keep this a flat string array.
 */

export const TYPED_COMMAND_NAMES = [
  // Agent / Chat
  "agent_secret_status",
  "agent_secret_set",
  "agent_secret_clear",
  "agent_provider_test",
  "agent_engine_turn",
  "agent_engine_cancel",
  "agent_chat_load",
  "agent_chat_save",
  "agent_workspace_tool",
  "agent_patch_get",
  "agent_patch_apply",
  "agent_patch_reject",
  "agent_patch_rollback",
  "agent_patch_summary",
  "agent_recipe_list",
  "agent_recipe_run",
  "agent_recipe_suggest",
  "agent_recipe_cancel",
  "agent_delegate_brief",
  "agent_delegate_record_launch",
  "agent_handoff_approve",
  // Process / PTY
  "pty_create",
  "pty_write",
  "pty_resize",
  "pty_kill",
  "pty_kill_all",
  "open_terminal",
  "open_agent_cli",
  "run_command_preset",
  // OAuth (used)
  "oauth_config_status",
  "oauth_set_management_port",
  "oauth_clear_sidecar_client_key",
  "oauth_set_sidecar_enabled",
  "oauth_connection_status",
  "oauth_model_definitions",
  "oauth_start",
  "oauth_status",
  "oauth_submit_callback",
  "oauth_cancel",
  "oauth_open_url",
  "oauth_clear_management_secret",
  // LAN
  "lan_serve_start",
  "lan_serve_stop",
  "lan_serve_status",
  "lan_discover",
  "lan_list_last_peers",
  "lan_list_approved_packages",
  "lan_send_package",
  "lan_ask_peer",
  "lan_probe_presence",
  "lan_probe_address",
  "lan_chat_send",
  "lan_chat_list",
  "lan_pair_create",
  "lan_pair_list",
  "lan_pair_revoke",
  // Built-in proxy runtime
  "proxy_runtime_status",
  "proxy_runtime_start",
  "proxy_runtime_start_default",
  "proxy_runtime_stop",
  "proxy_management_config",
  "proxy_management_update",
  // Project / settings / destructive
  "get_settings",
  "save_settings",
  "get_projects_list",
  "add_project_to_list",
  "remove_project_from_list",
  "get_app_state",
  "save_app_state",
  "init_project_cmd",
  "get_project",
  "save_project",
  "delete_project_cmd",
  "reset_all_data_cmd",
  // Sessions
  "scan_agent_sessions",
  "scan_workspace_agent_sessions",
  "read_foreign_session_transcript",
  // Host / path
  "get_host_os",
  "validate_path",
  "open_folder",
] as const;

export type TypedCommandName = (typeof TYPED_COMMAND_NAMES)[number];

const TYPED_SET = new Set<string>(TYPED_COMMAND_NAMES);

export function isTypedCommandName(name: string): name is TypedCommandName {
  return TYPED_SET.has(name);
}
