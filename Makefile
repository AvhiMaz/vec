run:
	cargo run --example vec

test:
	cargo test

build:
	cargo build

docs:
	cargo doc --open

clean:
	cargo clean

check:
	cargo clippy

fmt:
	cargo +nightly fmt --all
