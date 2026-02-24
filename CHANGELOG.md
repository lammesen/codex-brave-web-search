# Changelog

All notable changes to this project are documented in this file.

The format is based on Keep a Changelog and this project follows SemVer.

## [0.1.0] - 2026-02-24

### Added
- Rust MCP server (`brave-web-search`) with three tools:
  - `brave_web_search`
  - `brave_web_search_help`
  - `brave_web_search_status`
- Codex-first request/response/error envelopes with `api_version`, trace metadata, and structured warnings.
- Brave endpoint support for `web`, `news`, `images`, and `videos`.
- Input normalization/validation pipeline for query, locale, safety, freshness, filters, and pagination.
- Retry policy with exponential backoff + jitter, Retry-After support, and per-attempt timeout.
- Response body size guard and robust upstream error parsing.
- In-memory request cache (TTL) and local token-bucket throttling.
- Output line/byte truncation controls with warning emission.
- Debug payload controls, request URL reporting, and capped raw payload preview.
- Endpoint status probing and per-endpoint diagnostics.
- Unit/property/contract/snapshot/wiremock integration tests.
- Live smoke tests for all Brave search types.
- Local automation scripts for register/uninstall/restore MCP config.
- `Justfile` quality gates (`fmt`, `clippy`, offline/live tests).
- Optional CI and release workflow templates.
- Cargo-fuzz scaffold for parser hardening.
