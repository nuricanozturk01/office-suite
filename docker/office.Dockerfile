# syntax=docker/dockerfile:1
#
# All-in-one image: postgres + redis + api + web in a single container, for a single
# `docker run`. This is a quick-start convenience, not how the stack is meant to scale or
# how it's hardened in production (the api image in api.Dockerfile is distroless/nonroot;
# this one necessarily runs postgres/redis/nginx as root). Use docker-compose.yml instead
# for a real deployment.
#
# Build from the repository root:
#   docker build -f docker/office.Dockerfile -t office .
# Run:
#   docker run -d -p 8080:8080 -v office-data:/data office
# Then open http://localhost:8080. JWT secret is generated on first start and persisted on
# the /data volume, so it survives restarts.

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

# Clones and builds wordcraft/gridcraft/deckcraft/pdfcraft from their own upstream repos at
# the commits pinned in scripts/build-embeds.sh. Needs network access during the build and is
# the slow stage; Docker layer caching keeps repeat builds cheap unless the pins change.
# Not pinned to the project's 1.96.0 toolchain (this stage never touches that workspace) —
# trunk 0.22.1 itself requires rustc >= 1.96.1.
FROM rust:1.96.1-bookworm AS embeds
RUN rustup target add wasm32-unknown-unknown \
    && cargo install trunk --locked
WORKDIR /src
COPY scripts/build-embeds.sh scripts/rebrand.sh ./scripts/
COPY scripts/embed-patches ./scripts/embed-patches
RUN ./scripts/build-embeds.sh

FROM rust:1.96.0-bookworm AS api-build
WORKDIR /src
ENV SQLX_OFFLINE=true
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/src/target,sharing=locked \
    cargo build --locked --release -p api \
    && cp target/release/api /usr/local/bin/api

FROM node:24-bookworm-slim AS web-build
WORKDIR /src
RUN corepack enable
COPY . .
COPY --from=wasm /src/packages/formula-engine/pkg ./packages/formula-engine/pkg
COPY --from=embeds /src/apps/web/public/apps ./apps/web/public/apps
RUN pnpm install --frozen-lockfile \
    && pnpm --filter @office/web build

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl gnupg \
    && install -d /usr/share/postgresql-common/pgdg \
    && curl -fsSL https://www.postgresql.org/media/keys/ACCC4CF8.asc \
       -o /usr/share/postgresql-common/pgdg/apt.postgresql.org.asc \
    && echo "deb [signed-by=/usr/share/postgresql-common/pgdg/apt.postgresql.org.asc] https://apt.postgresql.org/pub/repos/apt bookworm-pgdg main" \
       >/etc/apt/sources.list.d/pgdg.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends postgresql-18 redis-server nginx \
    && apt-get purge -y curl gnupg \
    && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/* /etc/nginx/sites-enabled/default

COPY --from=api-build /usr/local/bin/api /usr/local/bin/api
COPY --from=web-build /src/apps/web/dist /usr/share/nginx/html
COPY docker/office-nginx.conf /etc/nginx/conf.d/office.conf
COPY docker/office-entrypoint.sh /usr/local/bin/office-entrypoint.sh
RUN chmod +x /usr/local/bin/office-entrypoint.sh

VOLUME /data
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/office-entrypoint.sh"]
