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

# Copy the example settings file
COPY settings.example.toml /home/prism/settings.toml

# Set the working directory
WORKDIR /home/prism

ENV RUST_LOG=debug
ENV PRISM_MSG_SETTINGS_FILE="/home/prism/settings.toml"

ENTRYPOINT ["prism-messenger-server"] 
