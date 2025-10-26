# --- Stage 1: Install cargo-chef using nightly (required for edition2024)
FROM rustlang/rust:nightly-slim-bullseye AS chef
RUN cargo +nightly install cargo-chef --locked
WORKDIR /sphinx-swarm

# --- Stage 2: Prepare the build plan
FROM rust:1.79-slim-bullseye AS planner
COPY --from=chef /usr/local/cargo/bin/cargo-chef /usr/local/cargo/bin/cargo-chef
WORKDIR /sphinx-swarm
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# --- Stage 3: Build dependencies and your app
FROM rust:1.79-slim-bullseye AS builder
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*

COPY --from=chef /usr/local/cargo/bin/cargo-chef /usr/local/cargo/bin/cargo-chef
WORKDIR /sphinx-swarm
COPY --from=planner /sphinx-swarm/recipe.json recipe.json

# Build dependencies
RUN cargo chef cook --release --recipe-path recipe.json

# Build the actual app
COPY . .
RUN cargo build --release --bin super

# --- Stage 4: Final runtime image (tiny and clean)
FROM debian:11-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy the built binary and static assets
COPY --from=builder /sphinx-swarm/target/release/super /usr/local/bin/super
COPY --from=builder /sphinx-swarm/src/bin/super/superapp/dist /sphinx-swarm/app/dist

# Set environment vars for Rocket
ENV ROCKET_ADDRESS=0.0.0.0 \
    ROCKET_PORT=8000

EXPOSE 8000
CMD ["super"]
