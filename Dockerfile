# Stage 1: Build
FROM rust:1.83-bookworm AS builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build only the server binary in release mode
RUN cargo build --release --package shepherd-server

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/shepherd-server /usr/local/bin/shepherd-server

EXPOSE 9876

ENV RUST_LOG=info

ENTRYPOINT ["shepherd-server"]
