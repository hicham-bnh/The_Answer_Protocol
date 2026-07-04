.PHONY: install run-server run-client run-client-gui lint clean

install:
	cargo check

run-server:
	cargo run -p server

run-client:
	cargo run -p client_cli

run-client-gui:
	cargo run -p client_gui

lint:
	cargo clippy --workspace -- -D warnings
	cargo fmt --all -- --check

clean:
	cargo clean