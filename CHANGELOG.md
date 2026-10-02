# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
