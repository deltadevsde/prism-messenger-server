# Build stage
FROM rust:1.86-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty shell project
WORKDIR /usr/src/app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Copy the actual source code
COPY src ./src
COPY settings.example.toml ./settings.example.toml

# Build the application
RUN cargo build --release

# Final stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary
COPY --from=builder /usr/src/app/target/release/prism-messenger-server /usr/local/bin/

# Set the working directory
WORKDIR /home/prism

ENV RUST_LOG=debug
ENV PRISM_MSG_SETTINGS_FILE="/home/prism/settings.toml"

ENV PRISM_MSG_WEBSERVER_HOST="127.0.0.1"
ENV PRISM_MSG_WEBSERVER_PORT=8080

ENV PRISM_MSG_PRISM_HOST="127.0.0.1"
ENV PRISM_MSG_PRISM_PORT=50020
ENV PRISM_MSG_PRISM_SIGNING_KEY_PATH="/home/prism/prism-signing-key.p8"

ENV PRISM_MSG_APNS_TEAM_ID="T1E234A5M"
ENV PRISM_MSG_APNS_KEY_ID="K12E34Y56"
ENV PRISM_MSG_APNS_PRIVATE_KEY_PATH="/home/prism/apns-auth-key.p8"
ENV PRISM_MSG_APNS_BUNDLE_ID="com.whatever.prism"

ENV PRISM_MSG_DATABASE_TYPE="sqlite"
ENV PRISM_MSG_DATABASE_PATH="/home/prism/prism_messenger.sqlite"

ENTRYPOINT prism-messenger-server -s "$PRISM_MSG_SETTINGS_FILE"
