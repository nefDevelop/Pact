import { safeStorage } from './crypto_utils.js';

/**
 * i18n ligero sin dependencias. Uso: `{i18n.t('vault.search')}` en plantillas y
 * `i18n.setLang('en')` para cambiar el idioma (persistido en localStorage).
 */
const messages = {
  es: {
    common: {
      cancel: 'Cancelar',
      save: 'Guardar',
      saving: 'Guardando...',
      close: 'Cerrar',
      edit: 'Editar',
      delete: 'Eliminar',
      restore: 'Restaurar',
      copy: 'Copiar',
      none: 'Ninguna',
      create: 'Crear',
      back: 'Volver',
      search: 'Buscar',
      copyUser: 'Usuario',
      copyPass: 'Contraseña',
      copyTotp: 'Código TOTP',
      copied: '{label} copiado al portapapeles',
      copyError: 'No se pudo copiar al portapapeles'
    },
    app: {
      settings: 'Configuración',
      users: 'Usuarios',
      noUsersTitle: 'Crea tu primer usuario para empezar.',
      emptyVaultTitle: 'Bóveda vacía: {name}',
      emptyVaultHint: 'Crea tu primer usuario en esta bóveda para continuar.',
      newUser: 'Crear Usuario',
      openVault: 'Abrir Bóveda',
      openVaultHint: 'Seleccionar archivo .pcty',
      createVault: 'Crear Nueva',
      createVaultHint: 'Empezar desde cero',
      dropVault: 'Suelta tu Bóveda',
      dropVaultHint: 'Solo archivos .pcty',
      change: 'Cambiar'
    },
    login: {
      locked: 'Sesión Bloqueada',
      masterPassword: 'Contraseña Maestra',
      placeholder: 'Introduce tu clave para entrar',
      unlock: 'Desbloquear Bóveda',
      verifying: 'Verificando...',
      argon: 'Argon2id Security',
      denied: 'Acceso denegado',
      incorrect: 'Contraseña incorrecta o usuario no válido.',
      failed: 'No se pudo iniciar sesión.'
    },
    footer: {
      syncNow: 'Sincronizar ahora',
      theme: 'Cambiar tema claro/oscuro',
      syncing: 'Sincronizando...',
      local: 'Local',
      online: 'Online',
      hybrid: 'Híbrido',
      light: 'Claro',
      dark: 'Oscuro'
    },
    vault: {
      quickAccess: 'Sesión Activa',
      addCredential: 'Agregar credencial',
      lock: 'Bloquear sesión',
      searchQuick: '¿Qué necesitas buscar ahora?',
      clearSearch: 'Limpiar búsqueda',
      openManager: 'Abrir gestor de bóveda',
      manager: 'Gestor de Bóveda',
      filterPlaceholder: 'Filtrar credenciales...',
      back: 'Volver al acceso rápido',
      settings: 'Abrir ajustes'
    },
    sidebar: {
      newItem: 'Nuevo Ítem',
      all: 'Todos',
      favorites: 'Favoritos',
      folders: 'Carpetas',
      newFolder: 'Nueva Carpeta',
      trash: 'Papelera',
      emptyTrash: 'Vaciar Papelera',
      emptyTrashConfirm: '¿Seguro que quieres vaciar la papelera?'
    },
    entry: {
      new: 'Nueva Entrada',
      edit: 'Editar Entrada',
      addSubtitle: 'Añadir cuenta al llavero',
      editSubtitle: 'Actualizar credencial guardada',
      service: 'Servicio',
      user: 'Usuario',
      password: 'Contraseña',
      generate: 'Generar frase segura',
      showPassword: 'Mostrar contraseña',
      hidePassword: 'Ocultar contraseña',
      weak: 'Contraseña vulnerable o muy común. Por favor, cámbiala.',
      totpLabel: 'Seed 2FA (TOTP)',
      totpInvalid: 'Formato Base32 Inválido',
      totpValid: 'Semilla Válida',
      paste: 'Pegar seed desde el portapapeles',
      pasteHint: 'Usa Ctrl+V (Navegador bloquea pegado automático)',
      advanced: 'Opciones avanzadas',
      website: 'Sitio Web (URL)',
      color: 'Color Identificador',
      category: 'Categoría',
      newCategory: 'Nueva',
      totpOn: '2FA activado',
      totpRemove: 'Quitar 2FA',
      totpKeep: 'Conservar',
      totpWillRemove: 'Se eliminará al guardar',
      saveEntry: 'Guardar Entrada',
      saveChanges: 'Guardar Cambios',
      addNow: 'Añadir Ahora'
    },
    settings: {
      title: 'Ajustes',
      users: 'Usuarios',
      categories: 'Categorías',
      system: 'Sistema',
      vault: 'Bóveda',
      security: 'Seguridad',
      language: 'Idioma',
      spanish: 'Español',
      english: 'English',
      autoLock: 'Bloqueo Automático',
      autoLockHint: 'Cerrar sesión por inactividad',
      never: 'Nunca',
      minutes: '{n} minutos',
      protected: 'Protegida',
      noCategories: 'Aún no hay categorías.',
      noCategoriesHint: 'Créalas desde la barra lateral de la bóveda.',
      local: 'Local',
      remote: 'Remoto',
      saveChanges: 'Guardar Cambios',
      unlinkAll: 'Desvincular Todo'
    },
    db: {
      path: 'Ruta .pcty',
      notLinked: 'Sin vincular',
      unlink: 'Desvincular',
      choose: 'Elegir',
      server: 'Servidor',
      name: 'DB',
      user: 'Usuario',
      password: 'Contraseña',
      connected: 'Lista',
      disconnected: 'Sin conexión',
      test: 'Probar Conexión'
    },
    secure: {
      deleteUser: 'Eliminar Usuario',
      exportUser: 'Exportar Usuario',
      masterKey: 'Clave Maestra',
      confirm: 'Confirmar'
    },
    folder: { title: 'Nueva Carpeta', subtitle: 'Categoriza tus credenciales', placeholder: 'Ej. Ocio, Trabajo...' },
    vaultModal: { title: 'Nueva Bóveda', subtitle: 'Elige un nombre para tu archivo', placeholder: 'Nombre de la bóveda' },
    userModal: { title: 'Nuevo Usuario', subtitle: 'Configurar identidad personal', alias: 'Alias de Usuario', masterPassword: 'Contraseña Maestra', identify: 'Identificador Visual', create: 'Crear Identidad' }
  },
  en: {
    common: {
      cancel: 'Cancel',
      save: 'Save',
      saving: 'Saving...',
      close: 'Close',
      edit: 'Edit',
      delete: 'Delete',
      restore: 'Restore',
      copy: 'Copy',
      none: 'None',
      create: 'Create',
      back: 'Back',
      search: 'Search',
      copyUser: 'Username',
      copyPass: 'Password',
      copyTotp: 'TOTP code',
      copied: '{label} copied to clipboard',
      copyError: 'Could not copy to clipboard'
    },
    app: {
      settings: 'Settings',
      users: 'Users',
      noUsersTitle: 'Create your first user to get started.',
      emptyVaultTitle: 'Empty vault: {name}',
      emptyVaultHint: 'Create your first user in this vault to continue.',
      newUser: 'Create User',
      openVault: 'Open Vault',
      openVaultHint: 'Select a .pcty file',
      createVault: 'Create New',
      createVaultHint: 'Start from scratch',
      dropVault: 'Drop your Vault',
      dropVaultHint: '.pcty files only',
      change: 'Change'
    },
    login: {
      locked: 'Session Locked',
      masterPassword: 'Master Password',
      placeholder: 'Enter your key to continue',
      unlock: 'Unlock Vault',
      verifying: 'Verifying...',
      argon: 'Argon2id Security',
      denied: 'Access denied',
      incorrect: 'Wrong password or invalid user.',
      failed: 'Could not sign in.'
    },
    footer: {
      syncNow: 'Sync now',
      theme: 'Toggle light/dark theme',
      syncing: 'Syncing...',
      local: 'Local',
      online: 'Online',
      hybrid: 'Hybrid',
      light: 'Light',
      dark: 'Dark'
    },
    vault: {
      quickAccess: 'Active Session',
      addCredential: 'Add credential',
      lock: 'Lock session',
      searchQuick: 'What do you need to find?',
      clearSearch: 'Clear search',
      openManager: 'Open vault manager',
      manager: 'Vault Manager',
      filterPlaceholder: 'Filter credentials...',
      back: 'Back to quick access',
      settings: 'Open settings'
    },
    sidebar: {
      newItem: 'New Item',
      all: 'All',
      favorites: 'Favorites',
      folders: 'Folders',
      newFolder: 'New Folder',
      trash: 'Trash',
      emptyTrash: 'Empty Trash',
      emptyTrashConfirm: 'Empty the trash?'
    },
    entry: {
      new: 'New Entry',
      edit: 'Edit Entry',
      addSubtitle: 'Add an account to the keyring',
      editSubtitle: 'Update saved credential',
      service: 'Service',
      user: 'Username',
      password: 'Password',
      generate: 'Generate secure passphrase',
      showPassword: 'Show password',
      hidePassword: 'Hide password',
      weak: 'Weak or very common password. Please change it.',
      totpLabel: '2FA Seed (TOTP)',
      totpInvalid: 'Invalid Base32 format',
      totpValid: 'Valid seed',
      paste: 'Paste seed from clipboard',
      pasteHint: 'Use Ctrl+V (browsers block auto-paste)',
      advanced: 'Advanced options',
      website: 'Website (URL)',
      color: 'Identifier Color',
      category: 'Category',
      newCategory: 'New',
      totpOn: '2FA enabled',
      totpRemove: 'Remove 2FA',
      totpKeep: 'Keep',
      totpWillRemove: 'Will be removed on save',
      saveEntry: 'Save Entry',
      saveChanges: 'Save Changes',
      addNow: 'Add Now'
    },
    settings: {
      title: 'Settings',
      users: 'Users',
      categories: 'Categories',
      system: 'System',
      vault: 'Vault',
      security: 'Security',
      language: 'Language',
      spanish: 'Español',
      english: 'English',
      autoLock: 'Auto Lock',
      autoLockHint: 'Sign out after inactivity',
      never: 'Never',
      minutes: '{n} minutes',
      protected: 'Protected',
      noCategories: 'No categories yet.',
      noCategoriesHint: 'Create them from the vault sidebar.',
      local: 'Local',
      remote: 'Remote',
      saveChanges: 'Save Changes',
      unlinkAll: 'Unlink All'
    },
    db: {
      path: '.pcty path',
      notLinked: 'Not linked',
      unlink: 'Unlink',
      choose: 'Choose',
      server: 'Server',
      name: 'DB',
      user: 'User',
      password: 'Password',
      connected: 'Ready',
      disconnected: 'Offline',
      test: 'Test Connection'
    },
    secure: {
      deleteUser: 'Delete User',
      exportUser: 'Export User',
      masterKey: 'Master Password',
      confirm: 'Confirm'
    },
    folder: { title: 'New Folder', subtitle: 'Categorize your credentials', placeholder: 'e.g. Leisure, Work...' },
    vaultModal: { title: 'New Vault', subtitle: 'Choose a name for your file', placeholder: 'Vault name' },
    userModal: { title: 'New User', subtitle: 'Set up personal identity', alias: 'User Alias', masterPassword: 'Master Password', identify: 'Visual Identifier', create: 'Create Identity' }
  }
};

function detectLang() {
  const saved = safeStorage.getItem('pact_lang');
  if (saved === 'es' || saved === 'en') return saved;
  const nav = (typeof navigator !== 'undefined' && navigator.language) || 'es';
  return nav.toLowerCase().startsWith('en') ? 'en' : 'es';
}

export const i18n = $state({
  lang: 'es',

  t(key, params) {
    const dict = messages[this.lang] || messages.es;
    let value = key.split('.').reduce((obj, part) => (obj ? obj[part] : undefined), dict);
    if (value === undefined) value = key;
    if (params && typeof value === 'string') {
      for (const [k, v] of Object.entries(params)) {
        value = value.replaceAll(`{${k}}`, String(v));
      }
    }
    return value;
  },

  setLang(lang) {
    if (lang !== 'es' && lang !== 'en') return;
    this.lang = lang;
    safeStorage.setItem('pact_lang', lang);
  }
});

i18n.lang = detectLang();
