ARGS ?= 127.0.0.1:8080

install:
	cargo build

run-client:
	cargo run -p client_cli -- $(ARGS)

run-client-gui:
	cargo run -p client_gui -- $(ARGS)

run-server:
	cargo run -p server

test:
	cargo test --workspace

lint:
	cargo clippy --workspace -- -D warnings
	cargo fmt --all -- --check

clean:
	cargo clean

.PHONY: install run-server run-client run-client-gui lint clean test
