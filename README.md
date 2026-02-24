# codex-brave-web-search

Rust MCP server for Codex with full Brave web/news/images/videos coverage and Codex-first structured responses.

- Server name: `brave-web-search`
- Tools:
  - `brave_web_search`
  - `brave_web_search_help`
  - `brave_web_search_status`

## Features

- Strict request parsing (`deny_unknown_fields`) with structured `INVALID_ARGUMENT` error envelopes.
- Brave endpoint support for `web`, `news`, `images`, and `videos`.
- Retry/backoff policy:
  - 3 retries (4 total attempts)
  - exponential backoff with jitter
  - max delay 5s
  - per-attempt timeout 15s
  - `Retry-After` support
- In-memory cache:
  - TTL 5 minutes
  - keyed by normalized request hash
  - bypassed when `freshness` is explicitly set
- Local throttling: 2 req/sec, burst 4.
- Output truncation defaults: 120 lines / 32KB.
- Per-call output overrides with bounded clamps:
  - min: 20 lines / 4KB
  - max: 300 lines / 96KB
- URL dedup based on normalized URL strategy for stable cross-section de-duplication.
- Debug controls with capped raw payload output (64KB cap).

## Build

```bash
cargo build --release --locked
```

Binary:

```text
target/release/codex-brave-web-search
```

## Register With Codex

### Automated (recommended)

```bash
sh scripts/register-mcp.sh
```

This script:

- builds release binary
- backs up any existing `brave-web-search` MCP config
- registers the new server
- prints rollback command

Uninstall:

```bash
sh scripts/uninstall-mcp.sh
```

Restore from backup:

```bash
sh scripts/restore-mcp-from-backup.sh <backup-json>
```

### Manual

```bash
codex mcp add brave-web-search -- "$(pwd)/target/release/codex-brave-web-search"
```

Inspect:

```bash
codex mcp get --json brave-web-search
```

Remove:

```bash
codex mcp remove brave-web-search
```

## Environment

### Required for live Brave requests

- `BRAVE_SEARCH_API_KEY` (preferred)
- fallback: `BRAVE_API_KEY`

Lookup order is `BRAVE_SEARCH_API_KEY`, then `BRAVE_API_KEY`.

### Runtime config (`CODEX_BRAVE_*`)

- Output limits:
  - `CODEX_BRAVE_DEFAULT_MAX_LINES`
  - `CODEX_BRAVE_DEFAULT_MAX_BYTES`
  - `CODEX_BRAVE_MIN_MAX_LINES`
  - `CODEX_BRAVE_MIN_MAX_BYTES`
  - `CODEX_BRAVE_MAX_MAX_LINES`
  - `CODEX_BRAVE_MAX_MAX_BYTES`
- Cache/throttle:
  - `CODEX_BRAVE_CACHE_TTL_SECS`
  - `CODEX_BRAVE_THROTTLE_RATE_PER_SEC`
  - `CODEX_BRAVE_THROTTLE_BURST`
- Retry/timeout/body caps:
  - `CODEX_BRAVE_RETRY_COUNT`
  - `CODEX_BRAVE_RETRY_BASE_DELAY_MS`
  - `CODEX_BRAVE_RETRY_MAX_DELAY_MS`
  - `CODEX_BRAVE_PER_ATTEMPT_TIMEOUT_MS`
  - `CODEX_BRAVE_MAX_RESPONSE_BYTES`
  - `CODEX_BRAVE_RAW_PAYLOAD_CAP_BYTES`
- Query cap:
  - `CODEX_BRAVE_MAX_QUERY_LENGTH`
- Logging:
  - `CODEX_BRAVE_LOG`
- Endpoint overrides:
  - `CODEX_BRAVE_ENDPOINT_WEB`
  - `CODEX_BRAVE_ENDPOINT_NEWS`
  - `CODEX_BRAVE_ENDPOINT_IMAGES`
  - `CODEX_BRAVE_ENDPOINT_VIDEOS`

## Tool Contract

### 1) `brave_web_search`

Request fields:

- Required: `query`
- Core optional: `search_type`, `result_filter` (array of strings), `max_results`, `offset`, `country`, `search_language`, `ui_language`, `safe_search`, `units`, `freshness`, `spellcheck`, `extra_snippets`, `text_decorations`
- `max_results` applies per returned section; for web with multiple `result_filter` sections, total returned results can exceed `max_results`.
- Output controls: `max_lines`, `max_bytes`
- Debug controls: `debug`, `include_raw_payload`, `disable_cache`, `disable_throttle`, `include_request_url`

Validation behavior:

- unknown fields: hard error
- empty query: hard error
- query > 2000 chars: truncates with warning
- invalid `search_type`: hard error
- invalid locale/safety/unit/freshness fields: warning + ignore
- `result_filter` for non-web: warning + ignore
- invalid `result_filter` tokens:
  - if at least one valid token exists: warning + ignore invalid tokens
  - if none are valid: hard error

Success envelope fields:

- top-level: `api_version`, `summary`, `sections`, `meta`, `warnings`
- optional `debug_data` when `debug=true`
- no score field

Error envelope fields:

- `api_version`
- `error.code`
- `error.message`
- optional `error.details`
- `meta.provider`, `meta.server_version`, `meta.trace_id`

Examples:

```json
{ "query": "TypeScript generics" }
```

```json
{ "query": "OpenAI", "search_type": "news", "max_results": 3 }
```

```json
{
  "query": "Kubernetes",
  "country": "US",
  "search_language": "en",
  "ui_language": "en-US",
  "result_filter": ["web", "discussions", "not-real"]
}
```

```json
{
  "query": "websocket server",
  "debug": true,
  "include_request_url": true,
  "include_raw_payload": true
}
```

### 2) `brave_web_search_help`

Request:

```json
{ "topic": "params|examples|limits|errors|all" }
```

Returns structured help sections and markdown examples.

### 3) `brave_web_search_status`

Request:

```json
{ "probe_connectivity": false, "verbose": false, "include_limits": false }
```

Notes:

- default `probe_connectivity=false`
- when enabled, probes all four Brave endpoints using query `mcp healthcheck`
- partial failures produce degraded status with per-endpoint diagnostics

## Testing

Offline deterministic path (no API key required):

```bash
cargo test -- --skip live_
```

Live Brave smoke tests (requires key):

```bash
cargo test --test live_smoke
```

Quality gates with `just`:

```bash
just verify-offline
just verify
```

`just verify` runs:

1. `cargo fmt --all`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. offline tests
4. live tests

## Fuzzing

Install cargo-fuzz once:

```bash
cargo install cargo-fuzz
```

Run targets:

```bash
cargo fuzz run parse_sections
cargo fuzz run parse_brave_error_message
```

Fuzz config lives in:

- `fuzz/Cargo.toml`
- `fuzz/fuzz_targets/`

## Optional CI/Release Templates

Templates are included but inactive by default:

- `.github/workflow-templates/ci.template.yml`
- `.github/workflow-templates/release-tag.template.yml`

Copy them into `.github/workflows/` to activate.
