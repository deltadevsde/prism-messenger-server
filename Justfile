check:
  @echo "Running cargo udeps..."
  cargo +nightly udeps --all-features --all-targets
  @echo "Running clippy..."
  cargo clippy --all --all-targets -- -D warnings

build:
  @echo "Building the project..."
  cargo build --release

try:
  @echo "Running the project..."
  RUST_BACKTRACE=1 RUST_LOG="debug,hyper=info,sqlx=info,aws=info" cargo run --release

unit-test:
  @echo "Running unit tests..."
  RUST_BACKTRACE=full cargo test --release -- --nocapture
