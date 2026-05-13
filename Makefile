# Pact: Unified Secure Password Manager (Rust + Tauri)
# -----------------------------------------------------------------------------

# Variables
NPM = npm
GUI_DIR = gui
CARGO = cargo
TAURI = npx @tauri-apps/cli

# Puertos configurables
API_PORT = 8002
GUI_PORT = 8088

.PHONY: help install install-gui test test-rs test-gui lint lint-gui format run-server dev clean build-tauri dev-tauri

help: ## Muestra este mensaje de ayuda
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

install: install-gui ## Instala todas las dependencias del proyecto

install-gui: ## Instala las dependencias de la interfaz (Svelte)
	cd $(GUI_DIR) && $(NPM) install

test: test-rs test-gui ## Ejecuta todos los tests (Rust y JS)

test-rs: ## Ejecuta tests de los crates de Rust (Core y Server)
	$(CARGO) test --workspace

test-gui: ## Ejecuta tests de la interfaz con Vitest
	cd $(GUI_DIR) && npx vitest run

# --- MODOS DE DESARROLLO ---

dev: ## Lanza Pact en modo Desarrollo (Rust API + Vite en navegador)
	@echo "Lanzando Pact en RUST DEV MODE..."
	@echo "Backend: http://localhost:$(API_PORT)"
	@echo "Frontend: http://localhost:$(GUI_PORT)"
	(trap 'kill 0' SIGINT; $(CARGO) run -p pact-server & cd $(GUI_DIR) && $(NPM) run dev -- --port $(GUI_PORT))

dev-tauri: ## Lanza Pact en modo Escritorio Nativo (Tauri + Svelte)
	@echo "Lanzando Pact en TAURI DEV MODE..."
	(trap 'kill 0' SIGINT; $(CARGO) run -p pact-server & cd $(GUI_DIR) && $(NPM) run dev -- --port $(GUI_PORT) & $(TAURI) dev)

# --- MODOS DE PRODUCCIÓN / BUILD ---

build-gui: ## Compila la interfaz para producción
	cd $(GUI_DIR) && $(NPM) run build

build-tauri: build-gui ## Compila el binario e instaladores nativos (AppImage, deb, exe)
	@echo "Construyendo binarios nativos con Tauri..."
	cd crates/pact-desktop && $(TAURI) build

run-server: build-gui ## Lanza el servidor de producción (Backend sirve la GUI)
	@echo "Lanzando Pact Server (Modo Producción)..."
	@echo "Bóveda accesible en: http://localhost:$(API_PORT)"
	$(CARGO) run -p pact-server --release

# --- CALIDAD Y LIMPIEZA ---

lint: lint-gui ## Ejecuta los linters en la interfaz
	cd $(GUI_DIR) && $(NPM) run lint
	$(CARGO) clippy -- -D warnings

check: check-rs check-gui ## Verifica errores en Rust y GUI (Svelte)

check-rs: ## Verifica compilación de Rust
	$(CARGO) check --workspace

check-gui: ## Verifica tipos y sintaxis en la GUI
	cd $(GUI_DIR) && npx svelte-check

format: ## Formatea el código de Rust y la GUI
	$(CARGO) fmt
	# cd $(GUI_DIR) && npx prettier --write . # Si está instalado

clean: ## Limpia archivos temporales y cachés
	$(CARGO) clean
	rm -rf $(GUI_DIR)/dist
	rm -rf $(GUI_DIR)/.svelte-kit
	rm -rf gui/node_modules
	rm -rf crates/pact-desktop/target
	rm -rf build/
