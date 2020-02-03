clean:
	rm -rf $(BIN_DIR)
	cargo clean

format:
	cargo fmt

build:
	cargo build --release

test:
	cargo test

package:
	cargo package

.PHONY: docs
docs:
	cargo doc --no-deps
	rm -rf ./docs
	cp -pr ./target/doc ./docs
