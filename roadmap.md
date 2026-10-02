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

### Próximos pasos
- [ ] Cobertura de código (`cargo tarpaulin`) y testing de mutación (`cargo mutants`) en CI
- [ ] Migraciones y cobertura de tests para MariaDB/MySQL remoto
- [ ] Empaquetar `pact-server` como sidecar de Tauri (`externalBin`) para la app instalada
- [ ] Fallback WebCrypto/WASM para el modo navegador
- [ ] i18n (es/en) y tests E2E de UI

