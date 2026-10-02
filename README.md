# Pact: Unified Secure Password Manager (Rust + Tauri)

Pact es un gestor de contraseñas y bóveda digital con **Cifrado E2E real** y un modelo de **conocimiento cero para los secretos**.

- La **MasterKey** se deriva en el cliente con Argon2id y **nunca se envía** al servidor; el servidor solo guarda un verificador de la `AuthKey`.
- Se cifran E2E en el cliente antes de salir del dispositivo: `title`, `url`, `username_site`, `encrypted_pass`, el **nombre de carpeta**, los **iconos** y la **semilla TOTP** (los códigos se generan en el cliente).
- El servidor **no puede descifrar** ninguno de esos campos ni generar codes TOTP.

**Límites actuales (no es ZK total):** el servidor todavía ve metadatos estructurales (número de cuentas, favoritos, papelera, versiones y marcas de tiempo) y la configuración de conexión remota (host, usuario y nombre de la BD). El cifrado del contenedor `.pcty` a nivel de archivo está pendiente.

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
- **Seguridad:** El servidor no ve los secretos ni las claves maestras; solo almacena blobs cifrados E2E y metadatos estructurales.

### 3. Interfaz (Tauri + Svelte)
- **Shell Nativa:** **Tauri v2** proporciona una ventana ligera y segura con acceso a APIs del sistema.
- **Frontend:** Construido con **Svelte 5** y **Tailwind CSS**, siguiendo los principios de **Material Design 3 (M3)**.
- **Tema/color:** la paleta tonal se deriva del color del usuario (`theme.js`); bordes, superficies y elevaciones usan tokens semánticos (`border-border`, `shadow-elev-*`, `--md-surface-*`) en vez de valores sueltos, y todo el acento respeta la semilla (`var(--md-primary)`).
- **i18n:** interfaz bilingüe (español/inglés) con un store propio sin dependencias y conmutador en Ajustes.
- **Iconos:** se sirven desde el backend mediante una allowlist de proveedores; el webview no contacta con terceros.

---

## Modelo de Seguridad y límites de Zero-Knowledge

Pact aplica **cifrado E2E de los secretos**: todo lo sensible se cifra en el
cliente con la `MasterKey` (AES-256-GCM) antes de salir del dispositivo. Sin
embargo, **no es un sistema Zero-Knowledge completo**: el servidor todavía
observa metadatos y parte de la configuración. Estos son los límites conocidos.

### Datos que el servidor NO puede leer
| Campo | Tratamiento |
|---|---|
| `accounts.title` / `url` / `username_site` / `encrypted_pass` | Cifrado E2E |
| `folders.name` | Cifrado E2E |
| `accounts.icon_data` | Cifrado E2E |
| `totp_settings.encrypted_seed` | Cifrado E2E; el código TOTP se genera en el cliente |
| `sync_configs.remote_pass` | Cifrado con una subclave derivada de la `MasterKey`; el servidor la descifra solo en tránsito durante el sync |
| `users.master_hash` | Verificador Argon2id de la `AuthKey` (no reversible a la `MasterKey`) |

### Datos que el servidor SÍ observa (pendiente)
| Dato | Riesgo | Mitigación propuesta |
|---|---|---|
| **Contenedor `.pcty` y BD SQLite sin cifrado de archivo** | Quien robe el `.pcty` obtiene todos los metadatos y los blobs E2E | Cifrado a nivel de archivo (SQLCipher o contenedor cifrado) |
| **Metadatos estructurales** (`folder_id`, `is_favorite`, `deleted_at`, `version`, `created_at`/`updated_at`, `has_totp`) | Nº de cuentas, favoritos, papelera, horarios y qué cuentas tienen 2FA | Ocultar/agrupar metadatos |
| **`sync_configs.remote_host` / `remote_user` / `remote_name`** | Infraestructura y usuario de la BD remota | Hacer la sincronización en cliente o documentarlo como decisión |
| **`users.username` / `color` / `created_at`** | Identidad y número de usuarios | Cifrar el alias de presentación |
| **`/auth/users` (pre-login)** | Listado de nombres de usuario | Limitar el listado o exigir un secreto local |
| **`/auth/salt/:username`** | ~~Enumeración~~ mitigado | Devuelve un salt determinista falso para usuarios inexistentes + rate limit por IP |

### Modelo de amenaza
- **Servidor/BD comprometidos**: no pueden descifrar secretos, nombres de carpeta,
  iconos ni seeds TOTP sin la `MasterKey`. Sí ven la estructura y la config de sync.
- **Robo del archivo `.pcty`**: expone metadatos; los secretos siguen protegidos
  por la `MasterKey` (sujeta a la fortaleza de la contraseña maestra y Argon2id).
- **Entrega del frontend**: en modo servidor (`make run-server`) el backend sirve
  el JS, por lo que un servidor malicioso podría intentar robar la `MasterKey`.
  En la app Tauri el bundle es local y hay CSP, lo que reduce este riesgo.

> El desarrollo pendiente para cerrar estas brechas está en
> [`roadmap.md`](roadmap.md) → *Próximos pasos*.

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

> El escritorio empaqueta `pact-server` como **sidecar** de Tauri
> (`externalBin`). Por eso `build-desktop` y `dev-tauri` ejecutan antes
> `make prepare-sidecar`, que compila el servidor y lo copia a
> `crates/pact-desktop/binaries/pact-server-<triple>`. Si compilas
> `pact-desktop` manualmente (`cargo build -p pact-desktop`) necesitas haber
> generado el sidecar primero.

---

## Verificación y Calidad

### Ejecutar Tests
```bash
make test             # Ejecuta todos los tests (Rust + Frontend)
make test-unit        # Tests unitarios de Rust (cripto + session/ratelimit/db)
make test-integration # Tests de integración (API, sync, storage, seguridad, MySQL)
make test-rs          # Todos los tests de Rust (Core + Server)
make test-gui         # Tests del frontend (Vitest)
make test-mysql       # Tests de MariaDB (requiere PACT_TEST_MYSQL_URL)
```

### Análisis de Código (Linting)
```bash
make lint         # Verifica estándares de código con Clippy, ESLint y svelte-check
make check        # Comprueba compilación de Rust y tipos de la GUI
```

### Integración Continua
El workflow de GitHub Actions (`.github/workflows/ci.yml`) ejecuta en cada push/PR:
- **Lint & tests**: `make lint`, `make test-unit`, `make test-integration` y `make test-gui`.
- **Desktop**: instala las dependencias de Tauri, prepara el sidecar y comprueba `pact-desktop`.
- **MariaDB**: levanta un servicio MariaDB y ejecuta `make test-mysql`.
- **Coverage / mutación**: genera cobertura con tarpaulin y publica el reporte de mutantes.

## Estructura del Proyecto

- `/crates/pact-core`: Lógica criptográfica compartida.
- `/crates/pact-server`: Servidor API en Rust.
- `/crates/pact-desktop`: Shell nativa de Tauri.
- `/gui`: Interfaz de usuario Svelte 5.
- `/migrations`: Esquemas de base de datos SQL.

---
**Pact Security Framework v2.0.0 | 2026**
