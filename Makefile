.PHONY: build test clean install fmt lint doc

all: build

build:
	cargo build --release

debug:
	cargo build

test:
	cargo test --workspace

clean:
	cargo clean

install:
	cargo install --path crates/opensam

fmt:
	cargo fmt --all

lint:
	cargo clippy --all-targets --all-features -- -D warnings

doc:
	cargo doc --no-deps --open

run:
	cargo run -- engage

init:
	cargo run -- init

check:
	cargo check --all
