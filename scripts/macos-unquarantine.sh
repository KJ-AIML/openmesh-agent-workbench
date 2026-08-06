#!/usr/bin/env bash
# Clear macOS Gatekeeper quarantine on an OpenMesh.app install.
#
# Preview GitHub Release DMGs are unsigned / not notarized. After download,
# macOS often shows "OpenMesh is damaged and can't be opened" (or blocks
# launch). That is quarantine + lack of Developer ID notarization — not a
# corrupted installer. This script removes the quarantine attribute so the
# app can launch for local dogfood.
#
# Usage:
#   ./scripts/macos-unquarantine.sh
#   ./scripts/macos-unquarantine.sh /Applications/OpenMesh.app
#   ./scripts/macos-unquarantine.sh ~/Downloads/OpenMesh.app
#
# Also works: right-click the app → Open → Open (once).
# Proper fix for end users: Apple Developer ID sign + notarize in CI.

set -euo pipefail

APP_PATH="${1:-/Applications/OpenMesh.app}"

if [[ ! -d "$APP_PATH" ]]; then
  echo "error: app bundle not found: $APP_PATH" >&2
  echo "Pass the path to OpenMesh.app (drag from Finder into Terminal)." >&2
  echo "Typical locations:" >&2
  echo "  /Applications/OpenMesh.app" >&2
  echo "  \$HOME/Downloads/OpenMesh.app" >&2
  exit 1
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "error: this script only runs on macOS" >&2
  exit 1
fi

echo "Clearing quarantine attributes on: $APP_PATH"
xattr -cr "$APP_PATH"

if xattr -l "$APP_PATH" 2>/dev/null | grep -q com.apple.quarantine; then
  echo "warning: com.apple.quarantine still present — try:" >&2
  echo "  sudo xattr -cr \"$APP_PATH\"" >&2
  exit 1
fi

echo "Done. Open with:"
echo "  open \"$APP_PATH\""
echo "If macOS still warns once: right-click → Open → Open."
