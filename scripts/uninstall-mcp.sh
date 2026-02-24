#!/bin/sh
set -eu

SERVER_NAME="${1:-brave-web-search}"
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
BACKUP_DIR="$REPO_ROOT/.mcp-backups"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_FILE="$BACKUP_DIR/${SERVER_NAME}-uninstall-${TIMESTAMP}.json"

if ! command -v codex >/dev/null 2>&1; then
  echo "Missing required command: codex" >&2
  exit 1
fi

mkdir -p "$BACKUP_DIR"

if codex mcp get --json "$SERVER_NAME" > "$BACKUP_FILE" 2>/dev/null; then
  codex mcp remove "$SERVER_NAME" >/dev/null
  echo "Removed MCP server: $SERVER_NAME"
  echo "Rollback command:"
  echo "  sh \"$REPO_ROOT/scripts/restore-mcp-from-backup.sh\" \"$BACKUP_FILE\""
else
  rm -f "$BACKUP_FILE"
  echo "No MCP server named '$SERVER_NAME' is registered."
fi
