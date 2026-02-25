# --- Stage 1: Install cargo-chef using nightly (required for edition2024)
# Use a nightly tag to support the unstable edition2024 feature
FROM rustlang/rust:nightly-bullseye AS chef
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
# Build dependencies - this is the caching Docker layer!
RUN cargo +nightly chef cook --release --recipe-path recipe.json

# Build application
COPY . .
RUN cargo +nightly build --release --bin super

# --- Stage 4: Runtime (No change needed here)
FROM debian:11-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Install system dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        protobuf-compiler \
        curl \
        gnupg \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Add Node to builder stage
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs

# Copy entire frontend into path that matches WORKDIR below
COPY src/bin/super/superapp/ /sphinx-swarm/src/bin/super/superapp/
# Copy main app so superapp's relative imports (../../../../../app/src/...) resolve at build time
COPY app/ /sphinx-swarm/app/

# Install dependencies and build frontend (dist is generated here, not from host)
WORKDIR /sphinx-swarm/src/bin/super/superapp
RUN npm install -g yarn
RUN yarn install
RUN yarn run build

# Use the container-built dist (do not copy from host)
WORKDIR /sphinx-swarm
RUN mkdir -p /sphinx-swarm/app && cp -r /sphinx-swarm/src/bin/super/superapp/dist /sphinx-swarm/app/dist
COPY --from=builder /sphinx-swarm/target/release/super /usr/local/bin/super



ENV ROCKET_ADDRESS=0.0.0.0 \
    ROCKET_PORT=8000

EXPOSE 8000
CMD ["super"]
