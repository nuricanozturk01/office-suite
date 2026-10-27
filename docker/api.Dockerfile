# syntax=docker/dockerfile:1
#
# Build from the repository root:
#   docker build -f docker/api.Dockerfile -t repo.repsy.io/nuricanozturk/myoffice-docker/api:latest .

FROM rust:1.96.0-bookworm AS build
WORKDIR /src
# Query macros compile from the committed `.sqlx` cache, so the build needs no database.
ENV SQLX_OFFLINE=true
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/src/target,sharing=locked \
    cargo build --locked --release -p api \
    && cp target/release/api /usr/local/bin/api

# Distroless nonroot: uid 65532, no shell, no package manager.
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /usr/local/bin/api /usr/local/bin/api
ENV API_ADDR=0.0.0.0:8080
EXPOSE 8080
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/api"]
