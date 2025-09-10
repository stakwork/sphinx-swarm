FROM rust:1.78-slim-bullseye AS chef
RUN cargo install cargo-chef
WORKDIR /sphinx-swarm

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    protobuf-compiler && \
    rm -rf /var/lib/apt/lists/*

COPY --from=planner /sphinx-swarm/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .
RUN cargo build --release --bin super

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