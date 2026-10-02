# Pact: Unified Secure Password Manager (Rust + Tauri)

Pact es un gestor de contraseñas y bóveda digital diseñado bajo el paradigma de **Conocimiento Cero (Zero-Knowledge)** y **Cifrado E2E Real**. El sistema garantiza que la soberanía de las claves criptográficas resida exclusivamente en el cliente, eliminando cualquier posibilidad de acceso a los datos por parte del servidor.

## Arquitectura Técnica (v2.0)

La aplicación ha sido migrada de un prototipo en Python a una arquitectura nativa de alto rendimiento.

### 1. Núcleo Criptográfico (Rust Core)
El motor de seguridad reside en `crates/pact-core`, implementado en Rust para máxima velocidad y seguridad de memoria.
- **Derivación de Claves (KDF):** Utiliza **Argon2id** en el cliente para derivar la `MasterKey` y la `AuthKey`, garantizando resistencia contra ataques de GPU y ASICs.
- **Cifrado E2E:** Implementa **AES-256-GCM** autenticado. Los datos se cifran en el navegador/app antes de ser enviados.
- **Hardware Fingerprinting:** Protección adicional para la configuración local del dispositivo.

### 2. Backend (Rust Server)
Servidor de alto rendimiento construido con **Axum** y **SQLx**.
- **Persistencia:** Soporte nativo para **SQLite** (Local) y **MariaDB/MySQL** (Sincronización Remota).
- **Seguridad:** El servidor es "ciego" (Zero-Knowledge); solo almacena blobs cifrados y nunca ve las claves maestras.

### 3. Interfaz (Tauri + Svelte)
- **Shell Nativa:** **Tauri v2** proporciona una ventana ligera y segura con acceso a APIs del sistema.
- **Frontend:** Construido con **Svelte 5** y **Tailwind CSS**, siguiendo los principios de **Material Design 3 (M3)**.

---

## Guía de Inicio Rápido

### 1. Requisitos Previos
- **Rust** (Cargo)
- **Node.js 20.19+** y **npm**
- **Tauri Dependencies** (ver [tauri.app](https://tauri.app/v2/guide/prerequisites))

### 2. Instalación
```bash
make install
```

---

## Modos de Ejecución

Pact utiliza un `Makefile` unificado para todas las tareas:

### A. Modo Desarrollo (Navegador)
Ideal para desarrollo rápido de la interfaz:
```bash
make dev
```
- **Frontend:** [http://localhost:8088](http://localhost:8088)
- **Backend API:** [http://localhost:8002](http://localhost:8002)

> [!NOTE]
> La derivación de claves (Argon2id) se ejecuta vía Tauri, por lo que el registro
> y el desbloqueo solo funcionan en el shell nativo (`make dev-tauri`). El modo
> navegador sirve para trabajar en la interfaz.

### B. Modo Desktop Nativo (Tauri)
Lanza la aplicación con su ventana nativa y backend integrado:
```bash
make dev-tauri
```

### C. Compilación para Producción
Genera el binario del servidor y el paquete de escritorio para Linux (`.deb`):
```bash
make build          # Servidor + escritorio
make build-server   # Solo el binario standalone del servidor
make build-desktop  # Solo el paquete de escritorio (alias: make build-tauri)
```

---

## Verificación y Calidad

### Ejecutar Tests
```bash
make test             # Ejecuta todos los tests (Rust + Frontend)
make test-unit        # Tests unitarios de Rust (cripto + session/ratelimit/db)
make test-integration # Tests de integración (API, sync, storage, seguridad)
make test-rs          # Todos los tests de Rust (Core + Server)
make test-gui         # Tests del frontend (Vitest)
```

### Análisis de Código (Linting)
```bash
make lint         # Verifica estándares de código con Clippy, ESLint y svelte-check
make check        # Comprueba compilación de Rust y tipos de la GUI
```

### Integración Continua
El workflow de GitHub Actions (`.github/workflows/ci.yml`) ejecuta `make lint`,
`make test-unit`, `make test-integration` y `make test-gui` en cada push/PR.

## Estructura del Proyecto

- `/crates/pact-core`: Lógica criptográfica compartida.
- `/crates/pact-server`: Servidor API en Rust.
- `/crates/pact-desktop`: Shell nativa de Tauri.
- `/gui`: Interfaz de usuario Svelte 5.
- `/migrations`: Esquemas de base de datos SQL.

---
**Pact Security Framework v2.0.0 | 2026**
