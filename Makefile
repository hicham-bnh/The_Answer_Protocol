.PHONY: install run-server run-client run-client-gui lint clean

install:
	@echo "Téléchargement et vérification des dépendances Rust..."
	cargo check

run-server:
	@echo "Démarrage du serveur Rust..."
	cargo run -p server

run-client:
	@echo "Démarrage du client CLI Rust..."
	cargo run -p client_cli

run-client-gui:
	@echo "Démarrage du client GUI Rust..."
	cargo run -p client_gui

lint:
	@echo "Analyse statique du code (Clippy)..."
	cargo clippy --workspace -- -D warnings
	@echo "Vérification du formatage..."
	cargo fmt --all -- --check

clean:
	@echo "Nettoyage du dossier target (builds)..."
	cargo clean