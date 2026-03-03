run:
	cargo run --example raw_vec && 	cargo run --example append_vec

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
