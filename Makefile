# Pact: Unified Secure Password Manager (Rust + Tauri)
# -----------------------------------------------------------------------------

# Variables
NPM = npm
GUI_DIR = gui
CARGO = cargo
TAURI = npx --package @tauri-apps/cli tauri

# Puertos configurables
API_PORT = 8002
GUI_PORT = 8088

# Directorio de artefactos de Cargo (el contenedor lo sobreescribe con target-container)
CARGO_TARGET_DIR ?= target
# Triple del host para nombrar el sidecar de Tauri
TARGET_TRIPLE := $(shell rustc -Vv 2>/dev/null | sed -n 's/^host: //p')
EXEEXT := $(if $(findstring Windows_NT,$(OS)),.exe,)
SIDECAR_DIR = crates/pact-desktop/binaries
SIDECAR = $(SIDECAR_DIR)/pact-server-$(TARGET_TRIPLE)$(EXEEXT)

.PHONY: help install install-gui deps test test-rs test-gui test-unit test-integration coverage mutants lint format run-server dev clean dev-tauri \
	build build-gui build-server build-desktop build-tauri prepare-sidecar check check-rs check-gui audit test-mysql

help: ## Muestra este mensaje de ayuda
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-20s\033[0m %s\n", $$1, $$2}'

install: install-gui ## Instala todas las dependencias del proyecto

install-gui: ## Instala las dependencias de la interfaz (Svelte)
	cd $(GUI_DIR) && $(NPM) install

deps: ## Instala las dependencias de la GUI solo si faltan (node_modules)
	@if [ ! -d "$(GUI_DIR)/node_modules" ]; then \
		echo "Dependencias de la GUI ausentes. Ejecutando npm install..."; \
		cd $(GUI_DIR) && $(NPM) install; \
	fi

test: test-rs test-gui ## Ejecuta todos los tests (Rust y JS)

test-rs: ## Ejecuta tests de los crates de Rust (Core y Server)
	$(CARGO) test -p pact-core -p pact-server

test-gui: deps ## Ejecuta tests de la interfaz con Vitest
	cd $(GUI_DIR) && npx vitest run

test-unit: ## Ejecuta unitarias Rust (session, ratelimit, crypto, db)
	$(CARGO) test -p pact-server --test session_tests --test ratelimit_tests --test db_crud_tests
	$(CARGO) test -p pact-core

test-integration: ## Ejecuta tests de integración (API, sync, use_cases, storage, security)
	$(CARGO) test -p pact-server --test api_contract_tests --test sync_tests --test use_case_tests --test storage_tests --test security_tests --test mysql_tests

test-mysql: ## Tests de MySQL/MariaDB (requiere PACT_TEST_MYSQL_URL)
	@test -n "$$PACT_TEST_MYSQL_URL" || (echo "Define PACT_TEST_MYSQL_URL (p. ej. mysql://user:pass@127.0.0.1:3306/pact_test)"; exit 1)
	$(CARGO) test -p pact-server --test mysql_tests -- --nocapture

coverage: ## Genera reporte de cobertura con tarpaulin (requiere: cargo install cargo-tarpaulin)
	$(CARGO) tarpaulin --workspace --out Html --output-dir coverage

mutants: ## Ejecuta testing de mutación (requiere: cargo install cargo-mutants)
	$(CARGO) mutants --check

# --- MODOS DE DESARROLLO ---

dev: deps ## Lanza Pact en modo Desarrollo (Rust API + Vite en navegador)
	@echo "Lanzando Pact en RUST DEV MODE..."
	@echo "Backend: http://localhost:$(API_PORT)"
	@echo "Frontend: http://localhost:$(GUI_PORT)"
	(trap 'kill 0' SIGINT; $(CARGO) run -p pact-server & cd $(GUI_DIR) && $(NPM) run dev -- --port $(GUI_PORT))

dev-tauri: deps prepare-sidecar ## Lanza Pact en modo Escritorio Nativo (Tauri + Svelte)
	@echo "Lanzando Pact en TAURI DEV MODE..."
	-pkill -9 -f "pact-server" 2>/dev/null; sleep 1
	# Tauri empaqueta/arranca pact-server (sidecar) como hijo de la ventana.
	(trap 'kill 0' SIGINT; cd $(GUI_DIR) && $(NPM) run dev -- --port $(GUI_PORT) < /dev/null & cd crates/pact-desktop && npx --package @tauri-apps/cli tauri dev < /dev/null)

# --- MODOS DE PRODUCCIÓN / BUILD ---

build: build-server build-desktop ## Genera todas las versiones de producción (Servidor y Desktop)

build-gui: deps ## Compila la interfaz para producción
	@echo "Construyendo GUI..."
	cd $(GUI_DIR) && $(NPM) run build

build-server: build-gui ## Genera el binario del servidor independiente (Standalone Server)
	@echo "Construyendo Pact Server (Release)..."
	$(CARGO) build -p pact-server --release
	@mkdir -p build/server
	@cp $(CARGO_TARGET_DIR)/release/pact-server$(EXEEXT) build/server/
	@echo "✓ Binario del servidor generado en build/server/pact-server"

prepare-sidecar: ## Genera el binario del servidor que Tauri empaqueta como sidecar
	@echo "Preparando sidecar: $(SIDECAR)"
	$(CARGO) build -p pact-server --release
	@mkdir -p $(SIDECAR_DIR)
	@cp $(CARGO_TARGET_DIR)/release/pact-server$(EXEEXT) $(SIDECAR)

build-desktop: build-gui prepare-sidecar ## Genera la aplicación de escritorio nativa (DEB y Binario)
	@echo "Construyendo Pact Desktop (Tauri)..."
	# Desactivamos AppImage para evitar problemas de FUSE/linuxdeploy
	cd crates/pact-desktop && $(TAURI) build --verbose --bundles deb
	@mkdir -p build/desktop
	@echo "Copiando binario nativo y paquete DEB..."
	@cp $(CARGO_TARGET_DIR)/release/pact-desktop$(EXEEXT) build/desktop/pact-desktop-raw
	@find $(CARGO_TARGET_DIR)/release/bundle -maxdepth 2 -type f -name "*.deb" -exec cp {} build/desktop/ \;
	@echo "✓ Versiones generadas en build/desktop/ (DEB y pact-desktop-raw)"

build-tauri: build-desktop ## Alias de build-desktop (compatibilidad con el README)

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

lint: audit deps ## Ejecuta los linters y auditorías de seguridad
	cd $(GUI_DIR) && $(NPM) run lint
	# Se lintan Core y Server (pact-desktop requiere dependencias de sistema de Tauri)
	$(CARGO) clippy -p pact-core -p pact-server --all-targets -- -D warnings

audit: ## Escanea vulnerabilidades en dependencias (npm audit + cargo audit)
	@echo "--- Dependencias JS (npm audit) ---"
	cd $(GUI_DIR) && $(NPM) audit --audit-level=high || true
	@echo "--- Dependencias Rust (cargo audit) ---"
	@if command -v cargo-audit >/dev/null 2>&1 || cargo audit --version >/dev/null 2>&1; then \
		cargo audit; \
	else \
		echo "cargo-audit no está instalado. Instálalo con: cargo install cargo-audit"; \
	fi

check: check-rs check-gui ## Verifica errores en Rust y GUI (Svelte)

check-rs: ## Verifica compilación de Rust (Core y Server)
	$(CARGO) check -p pact-core -p pact-server --all-targets

check-gui: deps ## Verifica tipos y sintaxis en la GUI
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
	rm -rf crates/pact-desktop/binaries
	rm -rf build/
