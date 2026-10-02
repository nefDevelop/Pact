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

### Fase 13: Sistema de color y jerarquía visual ✅
- [x] Tokens semánticos de borde (`border-border`), superficies (`--md-surface-*`) y elevación (`shadow-elev-1/2/3`) en `theme.js` + `app.css`
- [x] Escalera tonal correcta: lienzo fuera de la escalera de superficies (claro 91% / oscuro 9%) para que nada se pise
- [x] Elevación visible pero contenida: sombras tintadas con el hue de la semilla en claro; realce `inset` en oscuro
- [x] Contraste de textos secundarios con `--md-on-surface-variant` (sustituye `opacity-20/30/40` que desaparecía en oscuro)
- [x] Acento unificado a `var(--md-primary)`/`--md-on-primary` (fin de los `#6750A4` hardcodeados)
- [x] Eliminación de colores sueltos (`bg-white`, `bg-black/5`) a favor de tokens

### Fase 14: Preparación de producción (en curso)
Objetivo: poder desplegar Pact como app de escritorio (Linux/Windows) **y** como
servidor web, local o remoto, sin perder el modelo Zero-Knowledge.

**Seguridad y operación**
- [x] CORS por allowlist (orígenes Tauri, dev y web de producción); nunca permisivo
- [x] El sidecar de escritorio deja de arrancar con `PACT_CORS_ORIGIN=*` y con logs en `debug`
- [x] `/status` deja de exponer sesiones activas
- [x] `unwrap` en `unlock` sustituido por error explícito
- [x] TLS documentado para el modo web remoto (reverse proxy y alternativa nativa)
- [x] Revisar `/auth/users` y `/auth/salt` de cara a una exposición en red

**Calidad**
- [x] `console.*` del frontend acotados al modo desarrollo
- [x] `.env.example` y documentación de variables de entorno
- [x] `pact.machine` en el directorio de datos (no en el CWD)
- [x] `cargo audit` en `make audit`
- [x] `[profile.release]` con LTO, `codegen-units=1` y `strip`
- [ ] Umbral de cobertura y tests de componentes (login, modales)

**Modo web (WASM)**
- [x] `wasm_derivar_claves_dual` devuelve `master_key`/`auth_key`/`sync_key` en hex, igual que Tauri
- [x] `make build-wasm` encadenado en el build
- [x] Carga condicional en `crypto_utils.js` (`invoke` en Tauri, WASM en navegador)
- [x] Test de paridad WASM vs nativo con el mismo `password+salt` (`make test-wasm`)
- [x] README: el modo navegador deja de estar limitado a interfaz

**Release**
- [ ] Empaquetado Linux (AppImage) y Windows (NSIS/MSI)
- [x] Documentación de producción (instalación, datos, permisos, backup, servicio)
- [ ] Bump a 0.3.0 y tag

### Próximos pasos
- [ ] Cifrado a nivel de archivo del contenedor `.pcty` (SQLCipher o similar)
- [ ] Ocultar metadatos estructurales (nº de cuentas, favoritos, timestamps)
- [ ] Migraciones y cobertura de tests para MariaDB/MySQL remoto
- [ ] Tests E2E de UI

