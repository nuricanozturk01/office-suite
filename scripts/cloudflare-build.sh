#!/usr/bin/env bash
# Cloudflare Pages build for apps/web. The wasm engine is gitignored, so it is built here first.
set -euo pipefail

# The build may start in any directory (Workers Builds runs it from the Path setting), so go to the repo root.
cd "$(dirname "$0")/.."

WASM_BINDGEN_VERSION=0.2.129

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
. "$HOME/.cargo/env"
rustup target add wasm32-unknown-unknown

# Prebuilt CLI from the wasm-bindgen release; must match the `wasm-bindgen` crate version.
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
curl --proto '=https' --tlsv1.2 -sSfL \
  "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${WASM_BINDGEN_VERSION}/wasm-bindgen-${WASM_BINDGEN_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
  | tar -xz -C "$BIN_DIR" --strip-components=1 "wasm-bindgen-${WASM_BINDGEN_VERSION}-x86_64-unknown-linux-musl/wasm-bindgen"
export PATH="$BIN_DIR:$PATH"
wasm-bindgen --version

cargo build -q -p spreadsheets-engine-wasm --target wasm32-unknown-unknown --profile wasm
wasm-bindgen --target web --out-dir packages/formula-engine/pkg --out-name spreadsheets_engine_wasm \
  target/wasm32-unknown-unknown/wasm/spreadsheets_engine_wasm.wasm
pnpm install --frozen-lockfile
pnpm --filter @office/web build
