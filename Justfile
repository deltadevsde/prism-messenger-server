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
  RUST_BACKTRACE=full RUST_LOG="trace,ctclient::internal=off,reqwest=off,sp1_stark=info,jmt=off,p3_dft=off,p3_fri=off,sp1_core_executor=info,sp1_recursion_program=info,p3_merkle_tree=off,sp1_recursion_compiler=off,sp1_core_machine=off" cargo run

unit-test:
  @echo "Running unit tests..."
  RUST_BACKTRACE=full cargo test --release -- --nocapture
