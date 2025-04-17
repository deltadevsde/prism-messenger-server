# Build stage
FROM --platform=$BUILDPLATFORM rust:1.86-slim-bookworm AS builder

# Install required build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty shell project
WORKDIR /usr/src/app

# Copy only the files needed for dependency resolution
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to pre-download dependencies
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release

# Remove the dummy main.rs
RUN rm src/main.rs

# Copy the actual source code
COPY src ./src

# Build the application
RUN cargo build --release

# Final stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && update-ca-certificates

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/prism-messenger-server /usr/local/bin/

# Set the working directory
WORKDIR /home/prism

ENV RUST_LOG=debug
ENV PRISM_MSG_SETTINGS_FILE="/home/prism/settings.toml"

ENTRYPOINT ["prism-messenger-server"] 
