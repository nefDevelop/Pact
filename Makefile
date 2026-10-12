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
# Formatos de paquete del escritorio. Por defecto `rpm` (Fedora/RHEL), que es lo
# verificado en este entorno; en Debian/Ubuntu usa `make build-desktop BUNDLES=deb`.
BUNDLES ?= rpm
# Triple del host para nombrar el sidecar de Tauri
TARGET_TRIPLE := $(shell rustc -Vv 2>/dev/null | sed -n 's/^host: //p')
EXEEXT := $(if $(findstring Windows_NT,$(OS)),.exe,)
SIDECAR_DIR = crates/pact-desktop/binaries
SIDECAR = $(SIDECAR_DIR)/pact-server-$(TARGET_TRIPLE)$(EXEEXT)

.PHONY: help install install-gui deps test test-rs test-gui test-unit test-integration coverage mutants lint format run-server dev clean dev-tauri \
	build build-gui build-server build-desktop build-appimage build-windows build-tauri build-wasm prepare-sidecar check check-rs check-gui audit test-mysql test-wasm \
	seed-demo demo screens

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

test-integration: ## Ejecuta tests de integración (API, sync, use_cases, storage, security, seed)
	$(CARGO) test -p pact-server --test api_contract_tests --test sync_tests --test use_case_tests --test storage_tests --test security_tests --test coverage_gap_tests --test seed_demo_tests --test mysql_tests

test-wasm: ## Tests de paridad WASM vs nativo (requiere wasm-pack)
	@command -v wasm-pack >/dev/null 2>&1 || \
		(echo "Falta wasm-pack. Instálalo con: cargo install wasm-pack"; exit 1)
	wasm-pack test --node crates/pact-core

test-mysql: ## Tests de MySQL/MariaDB (requiere PACT_TEST_MYSQL_URL)
	@test -n "$$PACT_TEST_MYSQL_URL" || (echo "Define PACT_TEST_MYSQL_URL (p. ej. mysql://user:pass@127.0.0.1:3306/pact_test)"; exit 1)
	$(CARGO) test -p pact-server --test mysql_tests -- --nocapture

coverage: ## Reporte de cobertura con tarpaulin; falla si baja del 60% (requiere: cargo install cargo-tarpaulin)
	$(CARGO) tarpaulin -p pact-core -p pact-server --out Html --output-dir coverage --fail-under 60

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
	# El corchete en `[p]act-server` es necesario: sin él, el `pkill -f` también
	# casa con el propio shell de la receta y make se mata a sí mismo
	# ("Terminado (killed) (no tiene efecto)"). La caché de Vite se borra porque
	# si quedó desfasada tras editar un .svelte, vite-plugin-svelte no resuelve el
	# módulo virtual de estilos y @tailwindcss/vite acaba leyendo el fuente del
	# componente; el síntoma es ventana en blanco solo en dev.
	-pkill -9 -f "[p]act-server" 2>/dev/null; sleep 1
	@rm -rf $(GUI_DIR)/node_modules/.vite
	# Tauri empaqueta/arranca pact-server (sidecar) como hijo de la ventana.
	(trap 'kill 0' SIGINT; cd $(GUI_DIR) && $(NPM) run dev -- --port $(GUI_PORT) < /dev/null & cd crates/pact-desktop && npx --package @tauri-apps/cli tauri dev < /dev/null)

# --- MODO DEMO (capturas de pantalla) ---
# Aísla BD y bóveda en $(DEMO_DIR) para no tocar la del usuario. El servidor
# sirve la GUI desde `gui/dist`, relativo al directorio de trabajo, así que el
# objetivo se ejecuta desde la raíz del repo con la GUI ya compilada.

DEMO_DIR ?= data/demo
DEMO_PORT ?= 8092
DEMO_USER ?= demo
DEMO_PASS ?= DemoPass123!

seed-demo: ## Dataset cifrado de demo en $(DEMO_DIR) (5 carpetas, 21 cuentas)
	@mkdir -p $(DEMO_DIR)
	$(CARGO) run -q -p pact-server --bin seed_demo -- --data-dir $(DEMO_DIR) --username $(DEMO_USER) --password $(DEMO_PASS) --force

demo: build-gui seed-demo ## Levanta el servidor de demo con la bóveda de ejemplo
	@echo ""
	@echo "  Pact demo en http://localhost:$(DEMO_PORT)"
	@echo "  Usuario: $(DEMO_USER) / $(DEMO_PASS)"
	@echo ""
	DATABASE_URL="sqlite:$(DEMO_DIR)/pact.db" \
	PACT_VAULT_PATH="$(DEMO_DIR)/pact.pcty" \
	PACT_LISTEN_ADDR="127.0.0.1:$(DEMO_PORT)" \
	$(CARGO) run -q -p pact-server --bin pact-server

screens: build-gui seed-demo ## Igual que demo, reinicia el puerto y lista las pantallas
	-pkill -9 -f "[p]act-server" 2>/dev/null; sleep 1
	@echo ""
	@echo "  📸 Checklist de capturas (http://localhost:$(DEMO_PORT))"
	@echo ""
	@echo "    Selectores de usuario ......  abrir / y elegir 'demo'"
	@echo "    Desbloqueo .................  contraseña $(DEMO_PASS)"
	@echo "    Vista rápida ...............  vista de búsqueda + favoritos"
	@echo "    Gestor completo ...........  botón de abrir el gestor (icono de panel)"
	@echo "    Carpeta concreta ..........  sidebar: Personal / Trabajo / Finanzas / Desarrollo"
	@echo "    Favoritos .................  sidebar: Favoritos"
	@echo "    Papelera .................  sidebar: Papelera (3 entradas borradas)"
	@echo "    Alta de entrada ...........  botón + (o Ctrl+N)"
	@echo "    Alta de carpeta ...........  botón de carpeta en la sidebar"
	@echo "    Ajustes ...................  engranaje: Usuarios / Categorías / Sistema / Bóveda"
	@echo "    Bloqueo ...................  Ctrl+L"
	@echo "    Tema claro/oscuro .........  pie de página"
	@echo ""
	@echo "  El contenido está cifrado con la clave de la demo, así que la lista"
	@echo "  de entradas se ve sin descifrar nada a mano."
	@echo ""
	DATABASE_URL="sqlite:$(DEMO_DIR)/pact.db" \
	PACT_VAULT_PATH="$(DEMO_DIR)/pact.pcty" \
	PACT_LISTEN_ADDR="127.0.0.1:$(DEMO_PORT)" \
	$(CARGO) run -q -p pact-server --bin pact-server

# --- MODOS DE PRODUCCIÓN / BUILD ---

build: build-server build-desktop ## Genera todas las versiones de producción (Servidor y Desktop)

build-wasm: ## Compila pact-core a WASM para el KDF del navegador (requiere wasm-pack)
	@command -v wasm-pack >/dev/null 2>&1 || \
		(echo "Falta wasm-pack. Instálalo con: cargo install wasm-pack"; exit 1)
	@echo "Compilando pact-core a WASM..."
	wasm-pack build crates/pact-core --target bundler --release \
		--out-dir ../../gui/src/wasm --out-name pact_core --no-typescript
	@echo "✓ WASM generado en gui/src/wasm"

build-gui: deps build-wasm ## Compila la interfaz para producción
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

build-desktop: build-gui prepare-sidecar ## Empaqueta el escritorio en $(BUNDLES) (rpm por defecto; deb en Debian/Ubuntu)
	@echo "Construyendo Pact Desktop (bundles: $(BUNDLES))..."
	cd crates/pact-desktop && $(TAURI) build --verbose --bundles $(BUNDLES)
	@mkdir -p build/desktop
	@echo "Copiando binario nativo y paquetes..."
	@cp $(CARGO_TARGET_DIR)/release/pact-desktop$(EXEEXT) build/desktop/pact-desktop-raw
	@find $(CARGO_TARGET_DIR)/release/bundle -maxdepth 2 -type f \
		\( -name "*.rpm" -o -name "*.deb" -o -name "*.AppImage" \) -exec cp {} build/desktop/ \;
	@echo "✓ Paquetes en build/desktop/ ($(BUNDLES)) y binario en pact-desktop-raw"

build-appimage: build-gui prepare-sidecar ## Empaqueta AppImage. Requiere fs.inotify.max_user_instances > 128 (ver README)
	@echo "Construyendo Pact Desktop (AppImage)..."
	cd crates/pact-desktop && $(TAURI) build --verbose --bundles appimage
	@mkdir -p build/desktop
	@find $(CARGO_TARGET_DIR)/release/bundle -maxdepth 2 -type f -name "*.AppImage" -exec cp {} build/desktop/ \;
	@echo "✓ AppImage generado en build/desktop/"

build-tauri: build-desktop ## Alias de build-desktop (compatibilidad con el README)

build-windows: build-gui prepare-sidecar ## Empaqueta Windows (NSIS + MSI). Requiere ejecutarse en Windows
	@echo "Construyendo Pact Desktop (Windows)..."
	cd crates/pact-desktop && $(TAURI) build --verbose --bundles nsis,msi
	@mkdir -p build/desktop
	@find $(CARGO_TARGET_DIR)/release/bundle -maxdepth 2 -type f \( -name "*.exe" -o -name "*.msi" \) -exec cp {} build/desktop/ \;
	@echo "✓ Instaladores Windows generados en build/desktop/"

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
