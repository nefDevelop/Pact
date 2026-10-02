# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] - 2026-10-03

Correcciones de la ventana en blanco al abrir el AppImage, y del arranque cuando
el puerto de la API ya está ocupado. Sin cambios en el formato del contenedor ni
en el modelo Zero-Knowledge.

### Fixed
- **La ventana se quedaba en blanco al abrir el AppImage.** `Footer.svelte`
  seguía enlazando `onclick={sync_all}` a una función que el refactor del tema se
  había llevado por delante. Como `<Footer />` se renderiza de forma
  incondicional en la raíz de `App` (fuera de las ramas `{#if}`), el
  `ReferenceError` salía en el primer render y tumbaba el árbol entero: el
  servidor respondía, pero no se pintaba nada.
- **El servidor no arrancaba dentro del AppImage.** `DATABASE_URL` y
  `PACT_VAULT_PATH` tenían rutas relativas (`sqlite:pact.db`), y un AppImage se
  ejecuta desde su montaje squashfs de solo lectura, así que el sidecar moría con
  `Read-only file system (os error 30)` y dejaba la app sin API. Ahora ambas
  rutas por defecto apuntan al directorio de datos del usuario (o a
  `PACT_DATA_DIR`), igual que ya hacía el `machine-id` de reserva.
- **El tema no se decidía antes del primer pintado**, lo que provocaba un
  parpadeo claro→oscuro en el webview. La inicialización se movió a `main.js`
  (antes de `mount()`) y `Footer` solo cambia el tema cuando el usuario pulsa el
  conmutador.
- **El servidor no podía usarse dentro del AppImage con la CSP de producción.**
  El script en línea que fijaba el tema inicial estaba bloqueado por
  `script-src 'self'`, así que se eliminó en lugar de relajar la CSP.

### Added
- **Reutilización de un `pact-server` ya en marcha.** `start_server` lanzaba el
  sidecar sin mirar el puerto, así que con el 8002 ocupado moría con
  `Address already in use`, la app no se enteraba y el frontend acababa hablando
  con quien fuera que estuviera en ese puerto. Ahora se sondea `GET /status`
  antes de lanzar: si hay un Pact, se reutiliza; si hay otro servicio, no se
  lanza nada y se avisa. Además se comprueba que la API responde tras arrancar,
  en lugar de dejar una ventana que falla en silencio.
- La base de datos se crea con permisos `600` y se crea su directorio padre si
  falta.

### Known issues
- Si la app se cierra con `SIGTERM`, el `pact-server` queda huérfano ocupando el
  8002. Con la reutilización anterior es inofensivo, pero el proceso no termina.
- `svelte-check` no detecta referencias a identificadores inexistentes en los
  templates, porque el type-checking de los ficheros `.js` está desactivado.

## [0.3.0] - 2026-10-02

Ciclo de preparación de producción: endurece el despliegue, habilita el cliente
web y limpia la base de código. Sin cambios en el formato del contenedor ni en
el modelo Zero-Knowledge.

### Added
- **Cliente web funcional.** La derivación de claves en el navegador se hace
  con el mismo `pact-core` compilado a WASM (`make build-wasm`). Antes solo
  funcionaba dentro del shell de Tauri, porque `deriveKeys` llamaba al comando
  nativo de forma incondicional. Ambas rutas devuelven el mismo contrato
  (`master_key`/`auth_key`/`sync_key` en hex), de modo que una bóveda creada en
  un cliente se abre en el otro.
- Test de **paridad WASM vs nativo** (`make test-wasm`): deriva con ambos caminos
  un vector fijo y comprueba que coinciden byte a byte.
- Targets `build-wasm`, `test-wasm`, `build-appimage` y `build-windows`.
- `.env.example` con todas las variables que lee el servidor.
- Guía de despliegue: ubicación y permisos de los datos, backup coherente,
  reverse proxy con TLS y unidad systemd de ejemplo.
- `cargo audit` en `make audit`, junto al `npm audit` que ya existía.

### Security
- **CORS por allowlist.** El escritorio arrancaba el sidecar con
  `PACT_CORS_ORIGIN=*`, es decir CORS permisivo en la app distribuida. Eso
  permitía que cualquier página abierta en el navegador leyera los endpoints
  públicos (`/auth/users` enumera perfiles, `/auth/salt/:username`, `/status`).
  Ahora hay una lista explícita de orígenes (webview de Tauri, Vite dev,
  loopback) ampliable con `PACT_CORS_ORIGIN` como lista separada por comas; `*`
  se rechaza en lugar de degradar a permisivo.
- El nivel de log por defecto pasa a `info` (`tower_http=warn`), en vez de
  volcar cada request/response en la app distribuida.
- `/status` deja de informar de `active_sessions`: es público y el recuento
  permitía a una página local inferir si la app está en uso.
- El `machine-id` de reserva se escribe en el directorio de datos
  (`PACT_DATA_DIR` o el del usuario) con permisos `600`, en lugar del
  directorio de trabajo.
- `login` ya no usa `unwrap`: un cambio en la bandera de autorización no puede
  convertirse en un `panic` (500).
- Dependencias del frontend sin vulnerabilidades conocidas: `npm audit` pasa de
  16 avisos (1 crítico) a 0.

### Changed
- Perfil `release` con `lto`, `codegen-units=1` y `strip`; el sidecar queda en
  11 MB.
- Los diagnósticos del frontend pasan por un logger que solo escribe en
  desarrollo: `api.js` ya no vuelca cada URL en producción. Los errores reales
  (`console.error`) se mantienen.
- Se elimina `wasm-opt` del empaquetado: la versión de binaryen que trae
  `wasm-pack` rechaza las operaciones *bulk memory* del std actual.

### Fixed
- El navegador ya no falla al derivar claves (era el modo web entero).
- Se unifican los imports estáticos y dinámicos de `api.js` y
  `@tauri-apps/api/core`, que producían dos avisos de Rollup e impedían separar
  el bundle.

### Removed
- Dependencias de desarrollo sin uso: `postcss`, `autoprefixer`,
  `@vitest/browser` y `@tauri-apps/plugin-dialog` (el plugin de diálogo de Rust
  se mantiene; solo se retiró el paquete JS, que no se importaba).
- `@tailwindcss/vite` pasa a `devDependencies`; `dependencies` queda solo con lo
  que se usa en runtime.

[0.3.1]: https://example.com/pact/releases/tag/v0.3.1
[0.3.0]: https://example.com/pact/releases/tag/v0.3.0

## [0.2.0] - 2026-10-02

Primer ciclo de cierre y hardening. Salta de prototipo a release candidate.

### Added
- CRUD completo de cuentas en la GUI (crear/editar/eliminar) con cifrado E2E.
- Copia al portapapeles de usuario, contraseña y código TOTP.
- Pestaña de **Categorías** (editar/eliminar carpetas) en Ajustes.
- Auto-bloqueo por inactividad y auto-sincronización según modo local/remoto/híbrido.
- Propagación de borrados y restauraciones en el motor de sincronización.
- Generación **TOTP en el cliente** (HMAC-SHA1) a partir de un seed cifrado E2E.
- Cifrado E2E de nombres de carpeta e iconos.
- Empaquetado de `pact-server` como **sidecar** de Tauri (`externalBin`).
- CI de GitHub Actions (lint, tests Rust/GUI y build del escritorio).
- Tests de componentes Svelte y de endpoints adicionales.

### Changed
- `/sync/run` solo calcula el plan; `/sync/apply` lo aplica.
- Rate limiting de autenticación por **IP + usuario**.
- Los metadatos de la bóveda se anonimizan (solo nombre de archivo en `/auth/users`).
- `DATABASE_URL` solo admite SQLite local; el error es explícito.
- Documentación honesta del modelo Zero-Knowledge y sus límites.

### Security
- Autorización y verificación de propiedad en el borrado/restauración de recursos (IDOR).
- Corrección de path traversal en `upload_vault`/`initialize_vault`.
- Mitigación de SSRF en `proxy_icon` (resolución DNS, rangos privados, sin redirecciones, límite de tamaño).
- `get_vault_path`, `machine-id` y `pending-count` requieren sesión.
- Eliminado un `console.log` que filtraba la contraseña maestra.

### Fixed
- Seeds TOTP de 80 bits (Base32) ahora se decodifican correctamente.
- Renombrado/borrado de usuario consistente con la DB dentro del `.pcty`.
- `get_accounts` respeta `include_deleted`/`is_favorite`/`folder_id`.

[0.2.0]: https://example.com/pact/releases/tag/v0.2.0
