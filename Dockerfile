# Frontend build stage
FROM node:20-slim as frontend-builder

WORKDIR /app

# Copy frontend package files
COPY app/package.json ./

# Install dependencies (without frozen lockfile to resolve esbuild version mismatch)
RUN yarn install

# Copy frontend source code
COPY app/ ./

# Build the frontend
RUN yarn build

# Backend build stage
FROM rust:1.82-slim-bullseye as backend-builder

# Install system dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the Cargo manifest files
COPY Cargo.toml Cargo.lock ./

# Copy the source code
COPY src ./src

# Build the application for the stack binary
RUN cargo build --release --bin stack

# Runtime stage
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -u 1000 -m appuser

# Set the working directory
WORKDIR /app

# Copy the binary from the backend builder stage
COPY --from=backend-builder /app/target/release/stack /app/stack

# Copy the frontend dist files from the frontend builder stage
COPY --from=frontend-builder --chown=appuser:appuser /app/dist ./app/dist

# Copy any configuration files (if they exist in the root)
COPY --chown=appuser:appuser config*.yml ./
COPY --chown=appuser:appuser *.yml ./

# Create necessary directories and set permissions
RUN mkdir -p /vol /app/configs && \
    chown -R appuser:appuser /vol /app

# For Docker socket access, we need to run as root
# The Docker socket will be mounted with the host's docker group permissions
# USER appuser

# Expose the default port
EXPOSE 8000

# Set environment variables
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8000

# Run the binary
CMD ["./stack"]