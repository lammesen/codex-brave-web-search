# codex-brave-web-search

Brave Search MCP server for Codex (CLI/Desktop/IDE).

## Build & Test Commands

```sh
cargo build                              # Build
cargo test --all-features --locked       # Run all tests (unit + integration + doc-tests)
cargo test -- --skip live_               # Run offline tests only
cargo clippy --all-targets --all-features --locked -- -D warnings # Lint
cargo +nightly fmt --all --check         # Format check (nightly required)
cargo doc --no-deps --all-features --locked # Build docs
cargo deny check advisories bans licenses sources # Supply-chain policy
cargo audit --deny warnings              # RustSec advisory audit
```

## Code Style

- **Edition**: 2024, MSRV 1.85
- **Formatting**: `rustfmt.toml` with `max_width = 100`, use nightly rustfmt
- **Lints**: Clippy `pedantic` + `nursery` + `cargo` enabled, `unsafe_code` forbidden
- **All warnings are errors** in CI (`RUSTFLAGS="-Dwarnings"`)

## Commit Conventions

Use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` new feature (minor bump)
- `fix:` bug fix (patch bump)
- `docs:` documentation
- `chore:` maintenance/CI
- `refactor:` code restructuring
- `test:` tests
- `feat!:` or `BREAKING CHANGE:` footer for breaking changes (major bump)

## Project Structure

```
src/main.rs             # Binary entry point
src/lib.rs              # Library root
src/mcp_server.rs       # MCP server implementation
src/client.rs           # Brave API client
src/service.rs          # Search service logic
src/cache.rs            # Response caching
src/throttle.rs         # Rate limiting
src/parsing.rs          # Result parsing
src/formatting.rs       # Output formatting
src/normalization.rs    # Query normalization
src/config.rs           # Configuration
src/constants.rs        # Constants
src/error.rs            # Error types
src/types.rs            # Data types
tests/                  # Integration tests
fuzz/                   # Fuzz testing targets
scripts/                # Utility scripts
.github/workflows/      # CI/CD (ci, security, scorecard, release-plz, container)
```

## CI/CD Overview

- **ci.yml**: check, fmt, clippy, test, doc, machete (matrix: stable/nightly/MSRV)
- **security.yml**: cargo-audit + cargo-deny (advisories, bans, licenses, sources)
- **codeql.yml**: static analysis for Rust
- **dependency-review.yml**: blocks risky dependency changes in PRs
- **scorecard.yml**: OSSF Scorecard → GitHub Security tab
- **release-plz.yml**: auto Release PR + crates.io publish + GitHub Release
- **container.yml**: Docker build with cargo-chef → GHCR + provenance/SBOM + Trivy scan

## License

MIT OR Apache-2.0
