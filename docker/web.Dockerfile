# syntax=docker/dockerfile:1
#
# Self-contained build: formula-engine WASM, the four embedded reference editors
# (wordcraft/gridcraft/deckcraft/pdfcraft, cloned at their pinned commits), then the Vue SPA.
# No VITE_API_URL is baked in — the runtime nginx proxies /api/ to the `api` service, so the
# browser calls everything same-origin. Build from the repository root:
#   docker build -f docker/web.Dockerfile -t office-web .

FROM rust:1.96.0-bookworm AS wasm
WORKDIR /src
RUN rustup target add wasm32-unknown-unknown \
    && cargo install wasm-bindgen-cli --version 0.2.129 --locked
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build -q -p spreadsheets-engine-wasm --target wasm32-unknown-unknown --profile wasm \
    && wasm-bindgen --target web --out-dir packages/formula-engine/pkg \
       --out-name spreadsheets_engine_wasm \
       target/wasm32-unknown-unknown/wasm/spreadsheets_engine_wasm.wasm

# Builds wordcraft/gridcraft/deckcraft/pdfcraft from their own upstream repos at the commits
# pinned in scripts/build-embeds.sh. Needs network access during the build (git clone from
# github.com/storytold/*) and is the slow stage; Docker layer caching keeps repeat builds cheap
# as long as this file and the SHA pins don't change.
# Not pinned to the project's 1.96.0 toolchain (this stage never touches that workspace) —
# trunk 0.22.1 itself requires rustc >= 1.96.1.
FROM rust:1.96.1-bookworm AS embeds
RUN rustup target add wasm32-unknown-unknown \
    && cargo install trunk --locked
WORKDIR /src
COPY scripts/build-embeds.sh scripts/rebrand.sh ./scripts/
COPY scripts/embed-patches ./scripts/embed-patches
RUN ./scripts/build-embeds.sh

FROM node:24-bookworm-slim AS web
WORKDIR /src
RUN corepack enable
COPY . .
COPY --from=wasm /src/packages/formula-engine/pkg ./packages/formula-engine/pkg
COPY --from=embeds /src/apps/web/public/apps ./apps/web/public/apps
RUN pnpm install --frozen-lockfile \
    && pnpm --filter @office/web build

FROM nginxinc/nginx-unprivileged:1.27-alpine
COPY docker/nginx.local.conf /etc/nginx/conf.d/default.conf
COPY --from=web /src/apps/web/dist /usr/share/nginx/html
EXPOSE 8080
