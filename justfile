# DayRecord development commands

default:
    @just test

test:
    cargo test --workspace
    cd frontend && npm test

test-ignored:
    cargo test --workspace -- --ignored

fmt:
    cargo fmt --all

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

check:
    cargo check --workspace

build-app:
    cd frontend && npm run build
    cargo build -p dayrecord-app

run:
    cd frontend && npm run build
    cargo run -p dayrecord-app

cli:
    cargo build -p dayrecord-cli
    cargo run -p dayrecord-cli -- context --scope user

mcp:
    cargo run -p dayrecord-cli -- mcp

dev:
    cd frontend && npm run dev

ci:
    just fmt-check
    just clippy
    just test
    cd frontend && npm run typecheck

fmt-check:
    cargo fmt --all -- --check
