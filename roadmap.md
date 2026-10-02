# Pact Synchronization Roadmap

### Fase 1: Arquitectura y Modelos (Rust) ✅
- [x] Crear estructura del proyecto (`pact-server`)
- [x] Definir modelos de datos básicos (User, Folder, Account)
- [x] Implementar base de datos SQLite con Migraciones
- [x] Configurar Axum para endpoints básicos

### Fase 2: Autenticación y Seguridad ✅
- [x] Implementar registro e login con hashing de contraseña (Argon2)
- [x] Sistema de sesiones basado en tokens (DashMap en memoria)
- [x] Middleware de autenticación para proteger rutas de bóveda
- [x] Cifrado/Descifrado (Implementado con Argon2id + AES-256-GCM) ✅

### Fase 3: Motor de Sincronización (Backend) ✅
- [x] Diseñar el esquema de versionado (Incremental v1)
- [x] Implementar `SyncEngine` (Lógica de reconciliación)
- [x] Crear endpoints `/api/v1/sync/plan` y `/api/v1/sync/run`
- [x] Integrar con base de datos remota (MariaDB/MySQL)
- [x] Resolver conflictos básicos (Last Write Wins)

### Fase 4: Pruebas y Validación ✅
- [x] Crear tests de integración para sincronización bidireccional
- [x] Validar manejo de conflictos complejos (Concurrent edits)
- [x] Asegurar persistencia de metadatos (Version tags)

### Fase 5: Refinamiento y Shell Nativa (Tauri) 🔵
- [x] Inicializar Tauri como shell de escritorio principal
- [x] Migrar el "Bridge" (File Picker nativo) de Python a Tauri Commands (Rust)
- [x] Refactorizar cliente Svelte para usar el API de Rust via Localhost/Proxy

- [x] Implementar polling/webhooks para sync en tiempo real

### Fase 6: Consolidación y Despliegue 🔵
- [x] Auditoría de seguridad del cifrado E2E ✅
- [x] Eliminar dependencias de Python del entorno de usuario final ✅

### Fase 7: Cierre y Pulido ✅
- [x] CRUD de cuentas E2E en la GUI (crear/editar/eliminar) y copia al portapapeles
- [x] TOTP funcional (Base32 + URIs otpauth + cifrado en reposo)
- [x] Contrato de sincronización alineado (`/sync/run` calcula, `/sync/apply` aplica) y propagación de borrados
- [x] Auto-bloqueo por inactividad y auto-sync según modo local/remoto/híbrido
- [x] Endurecimiento de seguridad (auth/IDOR, path traversal, SSRF, cifrado TOTP, rate limit por IP)
- [x] Pestaña de Categorías, bandeja de pendientes, tema persistente, CSP y accesibilidad
- [x] CI (lint + tests) y tests de componentes Svelte

### Fase 8: Zero-Knowledge real para secretos ✅
- [x] Nombres de carpeta cifrados E2E (crear/editar/descifrar en cliente)
- [x] Iconos cifrados E2E en reposo
- [x] TOTP generado en el cliente; el servidor solo guarda el seed cifrado (`/vault/totp/:id` devuelve el blob)
- [x] Rutas de la bóveda anonimizadas (`/auth/users` solo nombre de archivo) y `get_vault_path`/`machine-id` requieren sesión
- [x] Documentación honesta del modelo ZK y sus límites

### Fase 9: Cierre de release ✅
- [x] Empaquetar `pact-server` como sidecar de Tauri (`externalBin`) y verificar el `.deb` (`usr/bin/pact-server` + `usr/bin/pact-desktop`)
- [x] Retirar la bandeja de pendientes muerta (sin productor) y exigir auth en `/vault/pending-count`
- [x] `DATABASE_URL` solo admite SQLite local con error explícito
- [x] CHANGELOG y bump de versión a 0.2.0
- [x] CI valida el build del escritorio (deps de Tauri + sidecar)

### Fase 10: Completar seguridad (Fase 2) ✅
- [x] Anti-enumeración en `/auth/salt` (salt determinista falso) + rate limit por IP en `/auth/salt` y `/auth/users`
- [x] Endurecimiento de `set_vault_path` (rechaza directorios y carpetas inexistentes)
- [x] `remote_pass` cifrada con una subclave derivada de la `MasterKey` (elimina el uso de `machine-id`); se descifra solo en tránsito con `X-Pact-Sync-Key`
- [x] Editar/borrar TOTP (residual de Fase 1)

### Fase 11: Completar UX (Fase 3) ✅
- [x] i18n propio (es/en) sin dependencias, con conmutador en Ajustes
- [x] Iconos de marca servidos por el backend (allowlist; el webview no contacta con terceros)
- [x] Accesibilidad: focus-trap en modales y `aria-label` en botones de solo icono

### Fase 12: Completar tests y CI (Fase 4) ✅
- [x] Esquema MySQL/MariaDB (`ensure_schema`) + tests de `MysqlDatabase` (target `make test-mysql`)
- [x] Job de CI con servicio MariaDB
- [x] Cobertura (tarpaulin) y testing de mutación (cargo-mutants) en CI
- [x] Tests de endpoints pendientes (`/vault/initialize`, etc.)

### Próximos pasos
- [ ] Cifrado a nivel de archivo del contenedor `.pcty` (SQLCipher o similar)
- [ ] Ocultar metadatos estructurales (nº de cuentas, favoritos, timestamps)
- [ ] Cobertura de código (`cargo tarpaulin`) y testing de mutación (`cargo mutants`) en CI
- [ ] Migraciones y cobertura de tests para MariaDB/MySQL remoto
- [ ] Fallback WebCrypto/WASM para el modo navegador
- [ ] i18n (es/en) y tests E2E de UI

