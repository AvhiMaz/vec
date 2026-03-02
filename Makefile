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

miri:
	cargo +nightly miri test

fmt:
	cargo +nightly fmt --all
