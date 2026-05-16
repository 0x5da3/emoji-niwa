# ── build ───────────────────────────────────────────────────────────────────
FROM rust:1-slim AS build
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml ./
# cache deps
RUN mkdir src && echo "fn main(){}" > src/main.rs && cargo build --release && rm -rf src
COPY src ./src
RUN touch src/main.rs && cargo build --release

# ── runtime ─────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/emoji-niwa-server /usr/local/bin/emoji-niwa-server
ENV BIND_ADDR=0.0.0.0:8080
ENV DATA_PATH=/data/state.json
EXPOSE 8080
CMD ["emoji-niwa-server"]
