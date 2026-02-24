#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <backup-json-file>" >&2
  exit 1
fi

BACKUP_FILE="$1"

if [ ! -f "$BACKUP_FILE" ]; then
  echo "Backup file not found: $BACKUP_FILE" >&2
  exit 1
fi

for cmd in codex jq; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Missing required command: $cmd" >&2
    exit 1
  fi
done

NAME=$(jq -r '.name // empty' "$BACKUP_FILE")
TRANSPORT_TYPE=$(jq -r '.transport.type // empty' "$BACKUP_FILE")

if [ -z "$NAME" ]; then
  echo "Backup file is missing .name" >&2
  exit 1
fi

if [ "$TRANSPORT_TYPE" != "stdio" ]; then
  echo "Only stdio transport backups are supported by this restore script." >&2
  exit 1
fi

COMMAND=$(jq -r '.transport.command // empty' "$BACKUP_FILE")
if [ -z "$COMMAND" ]; then
  echo "Backup file is missing .transport.command" >&2
  exit 1
fi

codex mcp remove "$NAME" >/dev/null 2>&1 || true

set -- codex mcp add "$NAME"

while IFS= read -r env_kv; do
  [ -n "$env_kv" ] || continue
  set -- "$@" --env "$env_kv"
done <<EOF_ENV
$(jq -r '.transport.env // {} | to_entries[] | "\(.key)=\(.value)"' "$BACKUP_FILE")
EOF_ENV

set -- "$@" -- "$COMMAND"

while IFS= read -r arg; do
  [ -n "$arg" ] || continue
  set -- "$@" "$arg"
done <<EOF_ARGS
$(jq -r '.transport.args[]?' "$BACKUP_FILE")
EOF_ARGS

"$@"

echo "Restored MCP server: $NAME"
