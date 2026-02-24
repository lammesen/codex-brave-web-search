set shell := ["sh", "-eu", "-c"]

default:
    @just --list

fmt:
    cargo fmt --all

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

test-offline:
    cargo test -- --skip live_

test-live:
    cargo test --test live_smoke

verify-offline:
    just fmt
    just clippy
    just test-offline

verify:
    just verify-offline
    just test-live
