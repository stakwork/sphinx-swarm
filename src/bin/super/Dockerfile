# --- Stage 1: Install cargo-chef using nightly (required for edition2024)
FROM rust:nightly-slim AS chef
RUN cargo +nightly install cargo-chef --locked
WORKDIR /sphinx-swarm

# --- Stage 2: Prepare the build plan
FROM chef AS planner
COPY . .
RUN cargo +nightly chef prepare --recipe-path recipe.json

# --- Stage 3: Build the app
FROM chef AS builder
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*

COPY --from=planner /sphinx-swarm/recipe.json recipe.json
RUN cargo +nightly chef cook --release --recipe-path recipe.json

# Build application
COPY . .
RUN cargo +nightly build --release --bin super

# --- Stage 4: Runtime
FROM debian:11-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /sphinx-swarm/target/release/super /usr/local/bin/super
COPY src/bin/super/superapp/dist /sphinx-swarm/app/dist

ENV ROCKET_ADDRESS=0.0.0.0 \
    ROCKET_PORT=8000

EXPOSE 8000
CMD ["super"]
