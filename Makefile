# Pact: Unified Secure Password Manager (Rust + Tauri)
# -----------------------------------------------------------------------------

# Variables
NPM = npm
GUI_DIR = gui
CARGO = cargo
TAURI = npx tauri

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
	-pkill -9 -f "pact-server" 2>/dev/null; sleep 1
	(trap 'kill 0' SIGINT; $(CARGO) run -p pact-server < /dev/null & cd $(GUI_DIR) && $(NPM) run dev -- --port $(GUI_PORT) < /dev/null & cd crates/pact-desktop && npx --package @tauri-apps/cli tauri dev < /dev/null)

# --- MODOS DE PRODUCCIÓN / BUILD ---

build: build-server build-desktop ## Genera todas las versiones de producción (Servidor y Desktop)

build-gui: ## Compila la interfaz para producción
	@echo "Construyendo GUI..."
	cd $(GUI_DIR) && $(NPM) run build

build-server: build-gui ## Genera el binario del servidor independiente (Standalone Server)
	@echo "Construyendo Pact Server (Release)..."
	$(CARGO) build -p pact-server --release
	@mkdir -p build/server
	@cp target/release/pact-server build/server/
	@echo "✓ Binario del servidor generado en build/server/pact-server"

build-desktop: build-gui ## Genera la aplicación de escritorio nativa (DEB y Binario)
	@echo "Construyendo Pact Desktop (Tauri)..."
	# Desactivamos AppImage para evitar problemas de FUSE/linuxdeploy
	cd crates/pact-desktop && $(TAURI) build --verbose --bundles deb
	@mkdir -p build/desktop
	@echo "Copiando binario nativo y paquete DEB..."
	@cp target-container/release/pact-desktop build/desktop/pact-desktop-raw || cp target/release/pact-desktop build/desktop/pact-desktop-raw
	@find target-container/release/bundle -maxdepth 2 -type f -name "*.deb" -exec cp {} build/desktop/ \; || \
	 find target/release/bundle -maxdepth 2 -type f -name "*.deb" -exec cp {} build/desktop/ \;
	@echo "✓ Versiones generadas en build/desktop/ (DEB y pact-desktop-raw)"

# --- PODMAN BUILD ---

CONTAINER_ENGINE = podman
IMAGE_NAME = pact-builder

build-container-image: ## Crea la imagen de podman para compilación
	@echo "Construyendo imagen de compilación: $(IMAGE_NAME)..."
	$(CONTAINER_ENGINE) build -t $(IMAGE_NAME) -f build.Containerfile .

build-in-container: build-container-image ## Ejecuta la compilación completa dentro de Podman
	@echo "Iniciando compilación dentro de Podman..."
	$(CONTAINER_ENGINE) run --rm \
		-v "$(CURDIR)":/app:Z \
		-v $(HOME)/.cargo/registry:/root/.cargo/registry:Z \
		-e CARGO_TARGET_DIR=target-container \
		$(IMAGE_NAME)

run-server: ## Lanza el servidor de producción (Backend sirve la GUI)
	@echo "Lanzando Pact Server (Modo Producción)..."
	@echo "Bóveda accesible en: http://localhost:$(API_PORT)"
	$(CARGO) run -p pact-server --release

# --- CALIDAD Y LIMPIEZA ---

lint: lint-gui audit ## Ejecuta los linters y auditorías de seguridad
	cd $(GUI_DIR) && $(NPM) run lint
	$(CARGO) clippy -- -D warnings

audit: ## Escanea vulnerabilidades en dependencias JS (npm audit)
	cd $(GUI_DIR) && $(NPM) audit --audit-level=high || true

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
