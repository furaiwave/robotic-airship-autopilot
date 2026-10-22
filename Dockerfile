# Multi-stage build: web UI (Node) + Rust API serving the UI and the REST API on :8080.
FROM node:24-bookworm-slim AS web
WORKDIR /src/apps/web
RUN corepack enable
COPY apps/web/package.json apps/web/pnpm-lock.yaml apps/web/pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile
COPY apps/web ./
RUN pnpm build

FROM rust:1-bookworm AS api
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release -p api-server --bin airship-api

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=api /src/target/release/airship-api /app/airship-api
COPY --from=web /src/apps/web/dist /app/web
ENV AIRSHIP_HOST=0.0.0.0 AIRSHIP_PORT=8080 AIRSHIP_STATIC_DIR=/app/web
EXPOSE 8080
CMD ["/app/airship-api"]
