.PHONY: test build

build:
	cargo build --release

test:
	cargo test

py:
	cargo build --lib --release --features "python-library"
	cp ./target/release/libreasonable.so reasonable.so
	python test.py

bench: build
	./scripts/bench_examples.sh
