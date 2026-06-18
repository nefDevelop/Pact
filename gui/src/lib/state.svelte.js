import { safeStorage } from './crypto_utils.js';
import { applyM3Palette } from './theme.js';

/**
 * @typedef {Object} User
 * @property {string} username
 * @property {string} [name]
 * @property {string} color
 */

/**
 * @typedef {Object} Notification
 * @property {string} id
 * @property {string} title
 * @property {string} message
 * @property {string} type
 */

export const globalState = $state({
  // --- UI STATE ---
  isDark: false,
  userColor: '#6750A4', 
  notifications: [],
  isDesktop: false,
  
  // --- VAULT & SYNC STATE ---
  isSyncing: false,
  hasConflicts: false,
  dbMode: 'local', // 'local' o 'remote'
  vaultPath: '',
  machineId: null,
  pendingCount: 0,
  trashCount: 0,
  lastSyncSuccess: Date.now(),
  syncError: false,
  lastSaveError: false,
  
  // --- AUTH STATE ---
  /** @type {User|null} */
  selectedUser: null,
  /** @type {CryptoKey|null} */
  masterKey: null, // Llave E2E (Solo en RAM)
  /** @type {string|null} */
  authToken: null, // Token de sesión Bearer

  // --- TIMERS ---
  autoLockMinutes: 5,
  remainingSeconds: 300,
  /** @type {any} */
  inactivityTimer: null,
  /** @type {any} */
  countdownInterval: null,
  /** @type {any} */
  syncInterval: null,
  /** @type {any} */
  pendingInterval: null,

  // --- ACTIONS ---

  updateTonalPalette(hex) {
    applyM3Palette(hex || this.userColor, this.isDark);
  },

  notify(title, message, type = 'info', duration = 5000) {
    const id = Math.random().toString(36).substring(7);
    this.notifications.push({ id, title, message, type });
    
    if (duration > 0) {
      setTimeout(() => {
        this.removeNotification(id);
      }, duration);
    }
  },
  
  removeNotification(id) {
    this.notifications = this.notifications.filter(n => n.id !== id);
  },

  forceLock(message) {
    if (typeof window !== 'undefined') {
      this.notify("Bóveda Bloqueada", message, "warning", 8000);
      window.dispatchEvent(new CustomEvent('pact:force-lock'));
    }
  },

  // --- LOGIC ---

  resetInactivityTimer() {
    if (typeof window === 'undefined') return;

    const saved = safeStorage.getItem('pact_autoLockMinutes');
    if (saved !== null) {
      this.autoLockMinutes = parseInt(saved, 10) || 5;
    }
    
    this.remainingSeconds = this.autoLockMinutes * 60;

    if (this.inactivityTimer) clearTimeout(this.inactivityTimer);
    if (this.countdownInterval) clearInterval(this.countdownInterval);

    if (this.autoLockMinutes <= 0) return;

    this.countdownInterval = setInterval(() => {
      if (this.remainingSeconds > 0) {
        this.remainingSeconds -= 1;
      }
    }, 1000);

    this.inactivityTimer = setTimeout(() => {
      this.forceLock("Sesión cerrada por inactividad.");
    }, (this.autoLockMinutes * 60 * 1000) + 500);

    safeStorage.setItem('pact_autoLockMinutes', String(this.autoLockMinutes));
  },

  updateDesktopStatus() {
    if (typeof window === 'undefined') return;
    const hasPyWebView = window.pywebview !== undefined || 
                         window.navigator.userAgent.includes('pywebview') ||
                         (window.external && window.external['pywebview'] !== undefined);
    
    const hasTauri = window.__TAURI__ !== undefined || !!window.__TAURI_INTERNALS__;
    
    this.isDesktop = hasPyWebView || hasTauri;
  },

  async refreshPendingCount() {
    const { api } = await import('./api.js');
    const u = this.selectedUser;
    if (!u || !u.username) {
      this.pendingCount = 0;
      return;
    }
    
    try {
      const data = await api.get(`/vault/pending-count/${u.username}`, { silent: true });
      this.pendingCount = data.count || 0;
    } catch (e) {
      console.warn("Fallo al refrescar pendientes:", e);
    }
  },

  async performSync(showNotify = false) {
    if (this.isSyncing) return;
    if (!this.authToken) return; // Evitar llamadas si no hay sesión
    this.isSyncing = true;
    
    const { api } = await import('./api.js');
    
    try {
      const plan = await api.post('/sync/run', { silent: true });
      this.lastSyncSuccess = Date.now();
      this.syncError = false;
      
      if (plan.conflicts && plan.conflicts.length > 0) {
        this.hasConflicts = true;
        this.notify("Conflicto de Datos", "Se han detectado versiones distintas. Haz clic en el icono de BD para resolver.", "warning", 0);
        return;
      }

      this.hasConflicts = false;

      if (plan.to_push.length > 0 || plan.to_pull.length > 0 || plan.to_push_folders.length > 0 || plan.to_pull_folders.length > 0) {
        await api.post('/sync/apply', plan);
        
        if (showNotify || this.dbMode === 'hybrid') {
          let msg = "Los datos están al día.";
          if (plan.to_push.length > 0 || plan.to_push_folders.length > 0) msg = "Cambios subidos a la nube.";
          if (plan.to_pull.length > 0 || plan.to_pull_folders.length > 0) msg = "Nuevos datos descargados.";
          this.notify("Sincronización Éxito", msg, "success");
        }
      }
    } catch (e) {
      if (e.message !== "Unauthorized") {
        console.error("Error en sync auto:", e);
      }
      this.syncError = true;
    } finally {
      this.isSyncing = false;
    }
  },

  async refreshTrashCount() {
    const { api } = await import('./api.js');
    try {
      const items = await api.get('/vault/accounts?include_deleted=true', { silent: true });
      this.trashCount = items.filter(i => !!i.deleted_at).length;
    } catch (e) {
      console.warn("Fallo al contar papelera:", e);
    }
  },

  startAutoSync() {
    this.performSync();
    if (this.syncInterval) clearInterval(this.syncInterval);
    
    this.syncInterval = setInterval(async () => {
      const { api } = await import('./api.js');
      try {
        const data = await api.get('/sync/check', { silent: true });
        if (data.has_changes) {
          this.performSync(true);
        }
      } catch (e) {
        console.warn("[SYNC] Error en polling:", e);
      }
    }, 30000);
  },

  startPendingPolling() {
    this.stopPendingPolling();
    this.refreshPendingCount();
    this.pendingInterval = setInterval(() => this.refreshPendingCount(), 5000);
  },

  stopPendingPolling() {
    if (this.pendingInterval) {
      clearInterval(this.pendingInterval);
      this.pendingInterval = null;
    }
  },

  stopAutoSync() {
    if (this.syncInterval) {
      clearInterval(this.syncInterval);
      this.syncInterval = null;
    }
  },

  resetSessionState() {
    this.pendingCount = 0;
    this.lastSaveError = false;
    this.selectedUser = null;
    this.masterKey = null;
    this.authToken = null;
    this.stopPendingPolling();
    this.stopAutoSync();
  },

  async initialize() {
    this.updateDesktopStatus();
    
    // Re-comprobación por si se inyecta tarde
    setTimeout(() => this.updateDesktopStatus(), 1500);

    const { api } = await import('./api.js');

    try {
      // 1. Machine ID
      const midData = await api.get('/system/machine-id', { silent: true });
      this.machineId = midData.machine_id;

      // 2. Solo intentamos cargar/enviar config de sync si tenemos token
      if (this.authToken) {
        const { decryptWithHardware } = await import('./crypto_utils.js');
        const encryptedConfig = safeStorage.getItem('pact_remote_v1');
        const savedConfig = await decryptWithHardware(encryptedConfig);

        if (savedConfig?.host) {
          this.dbMode = savedConfig.preferred_mode || 'remote';
          await api.post('/sync/config', {
            preferred_mode: savedConfig.preferred_mode || 'remote',
            host: savedConfig.host,
            user: savedConfig.user,
            password: savedConfig.password,
            db_name: savedConfig.db_name || 'pact_db'
          });
        }

        // 3. Sync Config
        const data = await api.get('/sync/config', { silent: true });
        if (data.preferred_mode) this.dbMode = data.preferred_mode;
      }

    } catch (e) {
      if (e.message !== "Unauthorized") {
        console.error("Error inicializando estado global:", e);
      }
    }
  }
});
