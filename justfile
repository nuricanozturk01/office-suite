# Query macros compile from the committed `.sqlx` cache. A fresh database has no
# tables yet, so a live check fails before startup migrations can create them.
export SQLX_OFFLINE := "true"

default: check

fmt:
    cargo fmt --all
    pnpm format

fmt-check:
    cargo fmt --all --check
    pnpm format:check

lint: wasm
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    pnpm lint

typecheck: wasm
    cargo check --workspace --all-targets
    pnpm typecheck

test: wasm
    cargo test --workspace
    pnpm test

arch:
    cargo run -q -p xtask -- arch

# Own target dir: flipping SQLX_OFFLINE in the shared one rebuilt every sqlx crate and left stale copies.
db-prepare:
    CARGO_TARGET_DIR=target/prepare SQLX_OFFLINE=false cargo sqlx prepare --workspace

# Writes `.env` on first run (random JWT secret). Shell exports still win.
env:
    #!/bin/sh
    set -eu
    if [ -f .env ]; then
        exit 0
    fi
    secret=$(openssl rand -hex 32)
    umask 077
    printf '%s\n' \
        'DATABASE_URL=postgres://office:office@localhost:5432/office' \
        "JWT_SECRET=${secret}" \
        'API_ADDR=127.0.0.1:8080' \
        'REDIS_URL=redis://127.0.0.1:6379' \
        'SQLX_OFFLINE=true' \
        > .env
    echo 'wrote .env'

dev-api: env
    cargo run -p api

# Browser calculation engine (Rust to WebAssembly) into packages/formula-engine/pkg. Needs the
# wasm32-unknown-unknown target and wasm-bindgen-cli 0.2.129 (the version of the `wasm-bindgen` crate).
wasm:
    cargo build -q -p spreadsheets-engine-wasm --target wasm32-unknown-unknown --profile wasm
    wasm-bindgen --target web --out-dir packages/formula-engine/pkg --out-name spreadsheets_engine_wasm target/wasm32-unknown-unknown/wasm/spreadsheets_engine_wasm.wasm

dev-web: wasm
    pnpm dev:web

dev-infra:
    docker compose -f docker/compose.yml up -d

# Prints an export line with a new 32-byte hex HMAC secret. Does not edit `.env`.
jwt:
    @printf 'export DATABASE_URL=postgres://office:office@localhost:5432/office JWT_SECRET=%s\n' "$(openssl rand -hex 32)"

check: fmt-check lint typecheck test arch vendor-check gen-api-check

# Needs a web build first (`pnpm build`).
budget:
    cargo run -q -p xtask -- budget

vendor-check:
    cargo run -q -p xtask -- vendor-check

db-migrate:
    cargo run -q -p xtask -- migrate

# Regenerates openapi.json from the Rust handlers, then the TypeScript client.
gen-api:
    cargo run -q -p api -- openapi > openapi.json.tmp && mv openapi.json.tmp openapi.json
    pnpm gen:api

# The Rust side is covered by the `committed_spec_is_up_to_date` test.
gen-api-check:
    pnpm --filter @office/api-client gen:check

# Frees disk: drops incremental caches and the db-prepare build (both regenerate on demand).
gc:
    rm -rf target/debug/incremental target/prepare
