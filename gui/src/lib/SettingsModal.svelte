<script>
  import { X, User, Shield, Bell, Languages, Clock, Save, Trash2, Database, Download, Server, HardDrive, Settings, Lock, ShieldAlert, Edit2, RefreshCw, Check, Search, Folder, Globe, ChevronRight, Type, Ghost, Gamepad2, Tv, Music, Coffee, Heart, Star, ShoppingBag, Luggage, Wallet, Umbrella, ArrowRightLeft, Plus, Loader2 } from 'lucide-svelte';
  import { globalState } from './state.svelte.js';
  import { encryptWithHardware, safeStorage } from './crypto_utils.js';
  import { untrack, tick } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import brandWhitelist from './brand_whitelist.json';

  const brandSet = new Set(brandWhitelist);

  const iconMap = {
    'folder': Folder, 'shield': Shield, 'lock': Lock, 'ghost': Ghost, 'gamepad2': Gamepad2,
    'tv': Tv, 'music': Music, 'coffee': Coffee, 'heart': Heart, 'star': Star,
    'shopping-bag': ShoppingBag, 'luggage': Luggage, 'wallet': Wallet, 'umbrella': Umbrella, 'trash-2': Trash2
  };

  let { isOpen = $bindable(), isUnlocked, folders = [], onRefreshFolders, onClose, users = $bindable() } = $props();

  let activeTab = $state('users');
  let language = $state('Español');
  let autoLock = $state('5 minutos');
  let isFetching = false;

  export async function refreshConfig() {
    if (isFetching) return;
    await loadSyncConfig();
  }

  let isSecureModalOpen = $state(false);
  let secureActionType = $state('delete');
  let targetUser = $state();
  let masterPasswordConfirm = $state('');

  let editingFolderId = $state(null);
  let editFolderName = $state('');
  let editFolderColor = $state('');

  const tabs = [
    { id: 'users', label: 'Usuarios', icon: User },
    { id: 'categories', label: 'Categorías', icon: Folder },
    { id: 'general', label: 'Sistema', icon: Settings },
    { id: 'database', label: 'Bóveda', icon: Database }
  ];

  async function saveFolderEdit() {
    if (!editFolderName) return;
    try {
      const response = await fetch(`/vault/folders/${editingFolderId}`, {
        method: 'PUT', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: editFolderName, color: editFolderColor })
      });
      if (response.ok) { editingFolderId = null; onRefreshFolders?.(); globalState.performSync(true); }
    } catch (error) { console.error('Error editando carpeta:', error); }
  }

  async function deleteFolder(id) {
    if (!confirm('¿Seguro que quieres eliminar esta categoría? Las cuentas no se borrarán.')) return;
    try {
      const response = await fetch(`/vault/folders/${id}`, { method: 'DELETE' });
      if (response.ok) { onRefreshFolders?.(); globalState.performSync(true); }
    } catch (error) { console.error('Error borrando carpeta:', error); }
  }

  let editingUserId = $state(null);
  let editUserName = $state('');
  let editUserColor = $state('');

  function startEdit(user) {
    editingUserId = user.id;
    editUserName = user.name;
    editUserColor = user.color;
  }

  async function saveUserEdit() {
    if (!editUserName) return;
    try {
      const oldUser = users.find(u => u.id === editingUserId);
      const response = await fetch(`/auth/update`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ old_name: oldUser.name, new_name: editUserName, new_color: editUserColor })
      });
      if (response.ok) {
        users = users.map(u => u.id === editingUserId ? { ...u, name: editUserName, color: editUserColor } : u);
        if (globalState.selectedUser && globalState.selectedUser.name === oldUser.name) {
          globalState.userColor = editUserColor;
          globalState.selectedUser.name = editUserName;
        }
        editingUserId = null;
        globalState.notify('Usuario', 'Cambios guardados correctamente.', 'success');
      }
    } catch (error) { console.error('Error actualizando usuario:', error); }
  }

  function openSecureAction(user, type) {
    targetUser = user;
    secureActionType = type;
    masterPasswordConfirm = '';
    isSecureModalOpen = true;
  }

  async function forceSync(username) {
    globalState.notify('Sincronización', `Forzando actualización para ${username}...`, 'info');
    await globalState.performSync(true);
  }

  let isWizardActive = $state(false);
  let accountsForWizard = $state([]);
  let wizardIndex = $state(0);
  let wizardUrl = $state('');
  let isWizardIconLoaded = $state(false);

  async function startIconWizard() {
    globalState.isSyncing = true;
    try {
      const resp = await fetch('/vault/accounts?include_deleted=false');
      if (resp.ok) {
        const allAccs = await resp.json();
        accountsForWizard = allAccs.filter(acc => {
          if (!acc.url) return true;
          const domain = acc.url.toLowerCase().trim().replace('https://', '').replace('http://', '').split('/')[0];
          return !brandSet.has(domain);
        });
        if (accountsForWizard.length > 0) {
          wizardIndex = 0;
          wizardUrl = accountsForWizard[0].url || '';
          isWizardActive = true;
        } else {
          globalState.notify('Identidad', 'Todas tus cuentas ya tienen identidades visuales optimizadas.', 'success');
        }
      }
    } catch (error) { console.error('Error cargando cuentas para asistente:', error); }
    finally { globalState.isSyncing = false; }
  }

  async function applyWizardUpdate(useLogo) {
    const acc = accountsForWizard[wizardIndex];
    globalState.isSyncing = true;
    try {
      const finalUrl = useLogo ? (wizardUrl || acc.title).toLowerCase().trim() : 'none';
      const response = await fetch(`/vault/accounts/${acc.id}`, {
        method: 'PUT', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ url: finalUrl })
      });
      if (response.ok) {
        isWizardIconLoaded = false;
        if (wizardIndex < accountsForWizard.length - 1) {
          wizardIndex++;
          wizardUrl = accountsForWizard[wizardIndex].url === 'none' ? '' : (accountsForWizard[wizardIndex].url || '');
        } else {
          isWizardActive = false;
          globalState.notify('Éxito', '¡Revisión de identidad completada!', 'success');
        }
      }
    } catch (error) { console.error('Error en asistente:', error); }
    finally { globalState.isSyncing = false; }
  }

  function handleWizardIconLoad() { isWizardIconLoaded = true; }
  function handleAvatarError(e) {
    const img = e.currentTarget;
    img.style.display = 'none';
    const next = img.nextElementSibling;
    if (next) next.style.display = 'flex';
  }

  let isLocalEnabled = $state(true);
  let isRemoteEnabled = $state(false);
  let isLocalConfigured = $derived(!!globalState.vaultPath);
  let isTestingConnection = $state(false);
  let isConnectionVerified = $state(false);
  let syncConfig = $state({ preferred_mode: 'local', host: '', user: '', password: '', db_name: '' });
  let hasLoadedInitialConfig = $state(false);
  let isInitialLoading = $state(false);

  async function loadSyncConfig() {
    if (isInitialLoading) return;
    isInitialLoading = true;
    try {
      const pathResp = await fetch('/vault/config/path');
      if (pathResp.ok) {
        const pathData = await pathResp.json();
        if (pathData.path) globalState.vaultPath = pathData.path;
      }
      const response = await fetch('/sync/config');
      if (response.ok) {
        const data = await response.json();
        syncConfig = { ...syncConfig, ...data };
        if (!hasLoadedInitialConfig) {
          if (syncConfig.preferred_mode === 'remote') { isRemoteEnabled = true; isLocalEnabled = false; }
          else if (syncConfig.preferred_mode === 'hybrid') { isLocalEnabled = true; isRemoteEnabled = true; }
          else if (syncConfig.preferred_mode === 'local' || globalState.vaultPath) { isLocalEnabled = true; isRemoteEnabled = !!syncConfig.host; }
          hasLoadedInitialConfig = true;
        }
      }
    } catch (error) { console.error('[⚙️ SETTINGS] Error crítico:', error); }
    finally { isInitialLoading = false; }
  }

  async function selectVaultFile(mode = 'open') {
    const hasTauri = typeof window !== 'undefined' && (window.__TAURI__ !== undefined || !!window.__TAURI_INTERNALS__);
    if (!globalState.isDesktop && !hasTauri) {
      globalState.notify('Sistema', 'Solo disponible en escritorio.', 'warning');
      return;
    }
    try {
      let path = null;
      if (hasTauri) {
        const { invoke } = await import('@tauri-apps/api/core');
        path = await invoke('browse_file', { mode });
      } else {
        /** @type {any} */
        const bridge = window['pywebview'];
        path = await bridge.api.browse_file(mode);
      }

      if (path) {
        const resp = await fetch('/vault/config/path', {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ path })
        });
        if (resp.ok) { globalState.vaultPath = path; globalState.notify('Bóveda', 'Éxito.', 'success'); }
      }
    } catch (e) { console.error(e); }
  }


  async function unlinkAll(confirmAction = true) {
    if (confirmAction && !window.confirm('¿Limpiar toda la configuración?')) return;
    try {
      // 1. Limpiamos configuración en el backend
      console.log("[⚙️ SETTINGS] Desvinculando ruta actual y limpiando config remota...");
      const resp = await fetch('/vault/config/path', { 
        method: 'POST', 
        headers: { 'Content-Type': 'application/json' }, 
        body: JSON.stringify({ path: null }) 
      });

      const empty = { preferred_mode: 'none', host: '', user: '', password: '', db_name: '' };
      await fetch('/sync/config', { 
        method: 'POST', 
        headers: { 
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${globalState.authToken}`
        }, 
        body: JSON.stringify(empty) 
      });

      // 2. Limpieza TOTAL del cliente
      console.log("[⚙️ SETTINGS] Realizando limpieza de almacenamiento local...");
      localStorage.clear();
      sessionStorage.clear();
      if (typeof window !== 'undefined' && window.indexedDB) {
        // Opcionalmente limpiar IndexedDB si se usara en el futuro
      }

      globalState.resetSessionState();
      globalState.vaultPath = '';
      syncConfig = empty;
      isLocalEnabled = true; 
      isRemoteEnabled = false; 
      isConnectionVerified = false;

      // 3. Forzamos el bloqueo y refresco de la UI
      window.dispatchEvent(new CustomEvent('pact:force-lock'));
      
      // Cerramos el modal de ajustes
      isOpen = false;
      onClose?.();

      // Forzar recarga de usuarios (ahora debería venir vacío)
      console.log("[⚙️ SETTINGS] Refrescando lista de usuarios (debería estar vacía)...");
      window.dispatchEvent(new CustomEvent('pact:refresh-users'));
      
      globalState.notify('Bóveda Reset', 'Se ha eliminado toda la configuración local y remota.', 'success');
    } catch (e) { 
      console.error("[⚙️ SETTINGS] Error crítico en reset:", e); 
    }
  }

  async function verifyConnection(manual = false) {
    if (!syncConfig.host || !syncConfig.user || !syncConfig.db_name) return;
    isTestingConnection = true;
    try {
      const response = await fetch('/sync/test', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(syncConfig) });
      isConnectionVerified = response.ok;
      if (response.ok && manual) globalState.notify('Conexión', '¡Éxito!', 'success');
    } catch (error) { isConnectionVerified = false; }
    finally { isTestingConnection = false; }
  }

  $effect(() => {
    if (hasLoadedInitialConfig) {
      const { host, user, password, db_name } = syncConfig;
      untrack(() => { isConnectionVerified = false; });
    }
  });

  async function saveDatabaseSettings() {
    if (!isLocalEnabled && !isRemoteEnabled) {
      await unlinkAll(false);
      isOpen = false;
      return;
    }

    let mode = 'local';
    if (isLocalEnabled && isRemoteEnabled) mode = 'hybrid';
    else if (isRemoteEnabled) mode = 'remote';

    if (!isLocalEnabled && globalState.vaultPath) {
      await fetch('/vault/config/path', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ path: null }) });
      globalState.vaultPath = '';
    }

    const payload = { ...syncConfig, preferred_mode: mode };
    try {
      const response = await fetch('/sync/config', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(payload) });
      if (response.ok) {
        globalState.notify('Bóveda', 'Guardado correctamente.', 'success');
        globalState.dbMode = mode;
        if (mode === 'hybrid' || mode === 'remote') globalState.performSync(false);
        window.dispatchEvent(new CustomEvent('pact:refresh-users'));
        isOpen = false;
      }
    } catch (error) { console.error(error); }
  }

  async function handleSecureAction() {
    if (!masterPasswordConfirm || !targetUser) return;
    globalState.isSyncing = true;
    try {
      if (secureActionType === 'delete') {
        const response = await fetch(`/auth/${targetUser.name}`, { method: 'DELETE', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ username: targetUser.name, master_password: masterPasswordConfirm }) });
        if (response.ok) {
          globalState.notify('Eliminado', 'Usuario borrado.', 'success');
          users = users.filter(u => u.name !== targetUser.name);
          isSecureModalOpen = false;
          if (globalState.selectedUser?.name === targetUser.name) { globalState.resetSessionState(); window.dispatchEvent(new CustomEvent('pact:force-lock')); }
        }
      } else {
        const response = await fetch('/vault/export', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ username: targetUser.name, master_password: masterPasswordConfirm }) });
        if (response.ok) {
          const blob = await response.blob();
          const url = window.URL.createObjectURL(blob);
          const a = document.createElement('a'); a.href = url;
          a.download = `pact_export_${targetUser.name}.json`;
          document.body.appendChild(a); a.click(); a.remove();
          isSecureModalOpen = false;
          globalState.notify('Exportar', 'Copia descargada.', 'success');
        }
      }
    } catch (error) { console.error(error); }
    finally { globalState.isSyncing = false; }
  }
</script>

{#if isOpen}
<div class="fixed inset-0 z-[100] flex items-center justify-center p-6 lg:p-12 overflow-hidden">
  <button aria-label="Cerrar modal" class="absolute inset-0 bg-black/60 backdrop-blur-md border-none cursor-default w-full h-full" onclick={onClose}></button>
  <div class="w-full max-w-[900px] h-[600px] bg-[var(--md-surface)] rounded-[48px] border border-black/[0.05] dark:border-white/[0.05] relative z-10 flex overflow-hidden shadow-2xl transition-colors duration-500">
    <!-- Sidebar -->
    <div class="w-20 lg:w-64 bg-[var(--md-surface-variant)] border-r border-black/[0.02] dark:border-white/[0.05] p-4 lg:p-8 flex flex-col gap-2">
      <h3 class="text-xl font-display font-black mb-6 px-2 hidden lg:block">Ajustes</h3>
      {#each tabs as tab}
        <button onclick={() => activeTab = tab.id} class="flex items-center justify-center lg:justify-start gap-3 px-4 py-4 lg:py-3 rounded-2xl transition-colors font-ui font-black text-[10px] uppercase tracking-widest {activeTab === tab.id ? 'text-white' : 'opacity-40 hover:opacity-100 hover:bg-black/5'}" style={activeTab === tab.id ? `background-color: ${globalState.userColor}` : ''} title={tab.label}><tab.icon size={18} /><span class="hidden lg:block">{tab.label}</span></button>
      {/each}
    </div>
    <!-- Content Area -->
    <div class="flex-1 flex flex-col overflow-hidden">
      <div class="p-8 pb-4 flex justify-between items-center shrink-0">
        <h4 class="text-[10px] font-ui font-black uppercase tracking-[0.2em] opacity-30">{tabs.find(t => t.id === activeTab)?.label || 'Ajustes'}</h4>
        <button onclick={onClose} class="opacity-40 hover:opacity-100" aria-label="Cerrar"><X size={20} /></button>
      </div>

      <div class="flex-1 overflow-y-auto p-8 pt-0 space-y-8 custom-scrollbar relative flex flex-col">
        <div class="flex-1">
          {#if activeTab === 'users'}
            <div class="space-y-4">
              {#each users as u}
                <div class="w-full h-20 bg-[var(--md-surface-variant)] rounded-[32px] px-6 flex items-center justify-between group border border-black/[0.02] dark:border-white/[0.05] hover:border-[var(--md-primary)]/20 transition-all">
                  <div class="flex items-center gap-4 flex-1">
                    <div class="w-12 h-12 rounded-full overflow-hidden border-2" style="border-color: {u.color}33"><img src={u.avatar} alt="avatar" class="w-full h-full object-cover" onerror={handleAvatarError} /></div>
                    <div class="flex flex-col flex-1">
                      {#if editingUserId === u.id}
                        <div in:fade={{ duration: 200 }} class="flex items-center gap-3 bg-[var(--md-surface)] p-2 rounded-2xl border border-[var(--md-outline-variant)]/30 shadow-sm">
                          <input bind:value={editUserName} class="bg-transparent border-none outline-none text-sm font-bold px-2 py-1 w-full max-w-[120px]" placeholder="Nombre..." onkeydown={(e) => e.key === 'Enter' && saveUserEdit()} aria-label="Editar nombre" />
                          <div class="flex items-center gap-2 shrink-0 border-l border-[var(--md-outline-variant)]/20 pl-3">
                            <input type="color" bind:value={editUserColor} class="w-6 h-6 rounded-full cursor-pointer border-none bg-transparent overflow-hidden" aria-label="Editar color" />
                            <button onclick={saveUserEdit} class="p-2 rounded-xl bg-green-500/10 text-green-600 hover:bg-green-500/20 transition-all active:scale-90" aria-label="Confirmar edición"><Check size={16} strokeWidth={3} /></button>
                          </div>
                        </div>
                      {:else}
                        <span class="text-sm font-bold tracking-tight">{u.name}</span>
                        <div class="flex items-center gap-2"><div class="w-1.5 h-1.5 rounded-full" style="background-color: {u.color};"></div><span class="text-[9px] opacity-30 uppercase font-black">Protegida</span></div>
                      {/if}
                    </div>
                  </div>
                  <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100">
                    <button onclick={() => forceSync(u.name)} class="p-2.5 rounded-xl hover:bg-blue-500/10 text-blue-500" aria-label="Forzar sincronización"><RefreshCw size={18} /></button>
                    <button onclick={() => startEdit(u)} class="p-2.5 rounded-xl hover:bg-black/5" aria-label="Editar usuario"><Edit2 size={18} /></button>
                    <button onclick={() => openSecureAction(u, 'export')} class="p-2.5 rounded-xl hover:bg-green-500/10 text-green-500" aria-label="Exportar datos"><Download size={18} /></button>
                    <button onclick={() => openSecureAction(u, 'delete')} class="p-2.5 rounded-xl hover:bg-red-500/10 text-red-500" aria-label="Eliminar usuario"><Trash2 size={18} /></button>
                  </div>
                </div>
              {/each}
            </div>
          {:else if activeTab === 'database'}
            <div class="space-y-8 pb-32">
              <div class="grid grid-cols-2 gap-4">
                <button onclick={() => isLocalEnabled = !isLocalEnabled} class="p-6 rounded-[32px] border border-[var(--md-outline-variant)]/50 dark:border-white/10 transition-all flex flex-col gap-3 {isLocalEnabled ? '' : 'opacity-40 grayscale'}" style={isLocalEnabled ? `border-color: ${globalState.userColor}; background-color: ${globalState.userColor}15; box-shadow: 0 0 20px ${globalState.userColor}15;` : ''} aria-label="Alternar almacenamiento local">
                  <div class="flex justify-between w-full"><HardDrive size={24} /><div class="w-10 h-5 rounded-full bg-black/10 relative transition-colors" style={isLocalEnabled ? `background-color: ${globalState.userColor}` : ''}><div class="absolute top-1 left-1 w-3 h-3 rounded-full bg-white transition-transform" style={isLocalEnabled ? 'transform: translateX(20px)' : ''}></div></div></div>
                  <div class="text-left"><span class="text-xs font-bold block">Local</span><span class="text-[9px] opacity-50 uppercase font-black">.pcty</span></div>
                </button>
                <button onclick={() => isRemoteEnabled = !isRemoteEnabled} class="p-6 rounded-[32px] border border-[var(--md-outline-variant)]/50 dark:border-white/10 transition-all flex flex-col gap-3 {isRemoteEnabled ? '' : 'opacity-40 grayscale'}" style={isRemoteEnabled ? `border-color: ${globalState.userColor}; background-color: ${globalState.userColor}15; box-shadow: 0 0 20px ${globalState.userColor}15;` : ''} aria-label="Alternar sincronización remota">
                  <div class="flex justify-between w-full"><Server size={24} /><div class="w-10 h-5 rounded-full bg-black/10 relative transition-colors" style={isRemoteEnabled ? `background-color: ${globalState.userColor}` : ''}><div class="absolute top-1 left-1 w-3 h-3 rounded-full bg-white transition-transform" style={isRemoteEnabled ? 'transform: translateX(20px)' : ''}></div></div></div>
                  <div class="text-left"><span class="text-xs font-bold block">Remoto</span><span class="text-[9px] opacity-50 uppercase font-black">MariaDB</span></div>
                </button>
              </div>
              {#if isLocalEnabled}
                <div class="p-8 rounded-[32px] border border-[var(--md-outline-variant)]/50 dark:border-white/10 bg-[var(--md-surface-variant)] space-y-4">
                  <span class="text-[9px] font-black uppercase opacity-30 flex justify-between items-center mb-1">Ruta .pcty</span>
                  <div class="flex gap-2"><input readonly value={globalState.vaultPath || 'Sin vincular'} class="flex-1 h-11 bg-[var(--md-surface)] rounded-xl px-4 text-xs font-mono opacity-60" aria-label="Ruta actual de la bóveda" /><button onclick={() => selectVaultFile('open')} class="h-11 px-4 rounded-xl bg-black/5 text-[10px] font-black uppercase tracking-widest">Elegir</button></div>
                </div>
              {/if}
              {#if isRemoteEnabled}
                <div class="p-8 rounded-[32px] border border-[var(--md-outline-variant)]/50 dark:border-white/10 bg-[var(--md-surface-variant)] space-y-4">
                  <div class="grid grid-cols-2 gap-4"><input bind:value={syncConfig.host} placeholder="Servidor" class="h-11 bg-[var(--md-surface)] rounded-xl px-4 text-xs" aria-label="Servidor de base de datos" /><input bind:value={syncConfig.db_name} placeholder="DB" class="h-11 bg-[var(--md-surface)] rounded-xl px-4 text-xs" aria-label="Nombre de la base de datos" /></div>
                  <div class="grid grid-cols-2 gap-4"><input bind:value={syncConfig.user} placeholder="Usuario" class="h-11 bg-[var(--md-surface)] rounded-xl px-4 text-xs" aria-label="Usuario de la base de datos" /><input type="password" bind:value={syncConfig.password} placeholder="Contraseña" class="h-11 bg-[var(--md-surface)] rounded-xl px-4 text-xs" aria-label="Contraseña de la base de datos" /></div>
                  <div class="flex justify-between items-center pt-2">
                    <div class="flex items-center gap-2"><div class="w-2 h-2 rounded-full {isConnectionVerified ? 'bg-green-500' : 'bg-red-500'}"></div><span class="text-[9px] font-black uppercase opacity-40">{isConnectionVerified ? 'Lista' : 'Sin conexión'}</span></div>
                    <button onclick={() => verifyConnection(true)} class="text-[9px] font-black uppercase tracking-widest text-[var(--md-primary)] bg-[var(--md-primary)]/10 hover:bg-[var(--md-primary)]/20 px-4 py-2 rounded-xl transition-all active:scale-95">Probar Conexión</button>
                  </div>
                </div>
              {/if}
            </div>
          {/if}
        </div>

        <!-- Botones de Acción Fijos Abajo (Solo en Tab Database) -->
        {#if activeTab === 'database'}
          <div class="absolute bottom-0 left-0 right-0 pt-8 pb-8 px-8 bg-gradient-to-t from-[var(--md-surface)] via-[var(--md-surface)] to-transparent z-20">
            <div class="flex justify-between items-center bg-[var(--md-surface-variant)]/50 backdrop-blur-md p-4 rounded-[32px] border border-[var(--md-outline-variant)]/20 shadow-xl">
              <button onclick={() => unlinkAll(true)} class="text-[10px] font-black uppercase text-red-500 px-6 h-12 rounded-2xl hover:bg-red-500/10 transition-all active:scale-95">Desvincular Todo</button>
              <button onclick={saveDatabaseSettings} class="px-8 h-12 rounded-2xl text-white font-bold text-[10px] uppercase tracking-[0.2em] shadow-lg active:scale-95 transition-all" style="background-color: {globalState.userColor};">Guardar Cambios</button>
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
  {#if isSecureModalOpen}
    <div class="absolute inset-0 z-[110] flex items-center justify-center p-8">
      <button aria-label="Cerrar modal de seguridad" class="absolute inset-0 bg-white/80 dark:bg-[#0b0a0d]/80 backdrop-blur-md w-full h-full border-none cursor-default" onclick={() => isSecureModalOpen = false}></button>
      <div class="w-full max-w-[400px] bg-white dark:bg-[#1f1e24] rounded-[40px] p-8 border {secureActionType === 'delete' ? 'border-red-500/20' : 'border-green-500/20'} relative z-10">
        <h5 class="text-xl font-bold mb-6 text-center">{secureActionType === 'delete' ? 'Eliminar Usuario' : 'Exportar Usuario'}</h5>
        <div class="space-y-6">
          <input type="password" bind:value={masterPasswordConfirm} class="w-full h-14 bg-black/5 rounded-2xl px-6 outline-none" placeholder="Clave Maestra" aria-label="Introduce tu clave maestra para confirmar" />
          <div class="flex gap-3">
            <button onclick={() => isSecureModalOpen = false} class="flex-1 h-14 rounded-full text-[10px] font-black uppercase opacity-40">Cancelar</button>
            <button onclick={handleSecureAction} class="flex-1 h-14 rounded-full text-white font-bold text-[10px] uppercase {secureActionType === 'delete' ? 'bg-red-500' : 'bg-green-600'}">Confirmar</button>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
{/if}
