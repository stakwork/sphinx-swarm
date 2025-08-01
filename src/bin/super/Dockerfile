# docker build -f src/bin/super/Dockerfile -t sphinx-super .
# or
# docker buildx build --platform linux/amd64 -f src/bin/super/Dockerfile -t sphinx-super .

# Rust as the base image
FROM rust:1.78-slim-bullseye AS build

# Create a new empty shell project
# RUN USER=root cargo new --lib sphinx-swarm
WORKDIR /sphinx-swarm

# Copy our manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# Build only the dependencies to cache them
# RUN cargo build --lib --release
# RUN rm src/*.rs

# Copy the source code
COPY ./src ./src

# install proto dep
RUN apt-get update && \
    apt-get install -y \
    protobuf-compiler 

# Build for release.
# remove the lib
# RUN rm ./target/release/deps/sphinx_swarm*
RUN cargo build --release --bin super

# The final base image
# FROM debian:buster-slim
# FROM debian:bullseye-slim
FROM debian:11-slim

# get root CA certs
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates 

# Copy from the previous build
COPY --from=build /sphinx-swarm/target/release/super /usr/src/super

# frontend assets
COPY src/bin/super/superapp/dist /sphinx-swarm/app/dist

# rocket
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8000

EXPOSE 8000

# Run the binary
CMD ["/usr/src/super"]