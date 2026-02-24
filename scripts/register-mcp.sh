#!/bin/sh
set -eu

SERVER_NAME="brave-web-search"
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
BINARY_PATH="${BINARY_PATH:-$REPO_ROOT/target/release/codex-brave-web-search}"
BACKUP_DIR="$REPO_ROOT/.mcp-backups"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_FILE="$BACKUP_DIR/${SERVER_NAME}-${TIMESTAMP}.json"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

need_cmd cargo
need_cmd codex

mkdir -p "$BACKUP_DIR"

echo "Building release binary..."
cargo build --release --locked --manifest-path "$REPO_ROOT/Cargo.toml"

if [ ! -x "$BINARY_PATH" ]; then
  echo "Expected binary not found: $BINARY_PATH" >&2
  exit 1
fi

if codex mcp get --json "$SERVER_NAME" > "$BACKUP_FILE" 2>/dev/null; then
  echo "Backed up existing MCP config to: $BACKUP_FILE"
  codex mcp remove "$SERVER_NAME" >/dev/null
else
  rm -f "$BACKUP_FILE"
fi

# Build the add command with optional inline environment variables.
set -- codex mcp add "$SERVER_NAME"

for name in \
  BRAVE_SEARCH_API_KEY \
  BRAVE_API_KEY \
  CODEX_BRAVE_DEFAULT_MAX_LINES \
  CODEX_BRAVE_DEFAULT_MAX_BYTES \
  CODEX_BRAVE_MIN_MAX_LINES \
  CODEX_BRAVE_MIN_MAX_BYTES \
  CODEX_BRAVE_MAX_MAX_LINES \
  CODEX_BRAVE_MAX_MAX_BYTES \
  CODEX_BRAVE_CACHE_TTL_SECS \
  CODEX_BRAVE_THROTTLE_RATE_PER_SEC \
  CODEX_BRAVE_THROTTLE_BURST \
  CODEX_BRAVE_RETRY_COUNT \
  CODEX_BRAVE_RETRY_BASE_DELAY_MS \
  CODEX_BRAVE_RETRY_MAX_DELAY_MS \
  CODEX_BRAVE_PER_ATTEMPT_TIMEOUT_MS \
  CODEX_BRAVE_MAX_RESPONSE_BYTES \
  CODEX_BRAVE_RAW_PAYLOAD_CAP_BYTES \
  CODEX_BRAVE_MAX_QUERY_LENGTH \
  CODEX_BRAVE_LOG \
  CODEX_BRAVE_ENDPOINT_WEB \
  CODEX_BRAVE_ENDPOINT_NEWS \
  CODEX_BRAVE_ENDPOINT_IMAGES \
  CODEX_BRAVE_ENDPOINT_VIDEOS
  do
  eval "val=\${$name:-}"
  if [ -n "$val" ]; then
    set -- "$@" --env "$name=$val"
  fi
done

set -- "$@" -- "$BINARY_PATH"
"$@"

echo
codex mcp get --json "$SERVER_NAME"

echo
if [ -f "$BACKUP_FILE" ]; then
  echo "Rollback command:"
  echo "  sh \"$REPO_ROOT/scripts/restore-mcp-from-backup.sh\" \"$BACKUP_FILE\""
else
  echo "No previous MCP config existed, so no rollback file was created."
fi

echo
if [ -z "${BRAVE_SEARCH_API_KEY:-}" ] && [ -z "${BRAVE_API_KEY:-}" ]; then
  echo "Warning: no BRAVE_SEARCH_API_KEY/BRAVE_API_KEY was captured into the MCP config." >&2
  echo "Set one and re-run this script if Brave auth is missing at runtime." >&2
fi
