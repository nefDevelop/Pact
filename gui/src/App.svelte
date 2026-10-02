<script>
  import LoginBox from './lib/LoginBox.svelte';
  import VaultView from './lib/VaultView.svelte';
  import UserList from './lib/UserList.svelte';
  import Footer from './lib/Footer.svelte';
  import NewUserModal from './lib/NewUserModal.svelte';
  import SettingsModal from './lib/SettingsModal.svelte';
  import NewEntryModal from './lib/NewEntryModal.svelte';
  import FolderModal from './lib/FolderModal.svelte';
  import VaultModal from './lib/VaultModal.svelte';
  import NotificationProvider from './lib/NotificationProvider.svelte';
  import { onMount, tick, untrack } from 'svelte';
  import { fly, fade } from 'svelte/transition';
  import { globalState } from './lib/state.svelte.js';
  import { api } from './lib/api.js';
  import { decryptE2E } from './lib/crypto_utils.js';
  import { i18n } from './lib/i18n.svelte.js';
  import { Settings, ChevronDown, Plus, File, Sun, Moon } from 'lucide-svelte';
  
  // Soporte para Tauri v2
  const isTauri = typeof window !== 'undefined' && !!(window.__TAURI_INTERNALS__);
  const invoke = isTauri
    ? (async () => (await import('@tauri-apps/api/core')).invoke) 
    : null;

  let users = $state([]);
  let isLoading = $state(true);
  let selectedUser = $state();
  let isMobile = $state(false);
  let isDropdownOpen = $state(false);
  let isModalOpen = $state(false);
  let isSettingsOpen = $state(false);
  let isEntryModalOpen = $state(false);
  let entryModalMode = $state('normal');
  let entryToEdit = $state(null);
  let folders = $state([]);
  let vaultRef = $state();
  let isUnlocked = $state(false);
  let showManager = $state(false);
  let isDragging = $state(false);

  // Flags de Modals
  let isNamingFolder = $state(false);
  let isNamingVault = $state(false);

  // Referencia al pill para posicionar el dropdown
  let pillRef = $state();
  let pillRect = $state(null);

  async function loadFolders() {
    try {
      const raw = await api.get('/vault/folders');
      // Los nombres de carpeta se cifran E2E; se descifran al cargarlos.
      folders = await Promise.all((raw || []).map(async (f) => {
        let name = f.name;
        if (globalState.masterKey && f.name) {
          const decrypted = await decryptE2E(f.name, globalState.masterKey);
          if (typeof decrypted === 'string' && decrypted) name = decrypted;
        }
        return { ...f, name };
      }));
    } catch (e) { /* Error manejado por api.js */ }
  }

  async function loadUsers(showToast = false) {
    globalState.isSyncing = true;
    users = [];
    try {
      const data = await api.get('/auth/users');
      const userList = data.users || [];
      // El modo de BD se determina tras autenticar (refreshSyncConfig), no aquí.
      globalState.vaultPath = data.vault_path || '';

      users = userList.map(u => ({
        id: u.id,
        name: u.username, // El alias elegido por el usuario
        username: u.username,
        status: u.status,
        color: u.color || '#6750A4',
        avatar: `https://api.dicebear.com/9.x/notionists/svg?seed=${u.username}`
      }));
      if (showToast && globalState.vaultPath) {
        globalState.notify('Bóveda', `Cargados ${users.length} usuarios.`, 'success', 3000);
      }

      if (users.length > 0) {
        if (!selectedUser || !users.find(u => u.name === selectedUser.name)) {
          selectedUser = users[0];
          globalState.selectedUser = selectedUser;
          globalState.userColor = selectedUser.color;
        }
        isModalOpen = false;
      } else {
        selectedUser = null;
        globalState.selectedUser = null;
        // Si hay boveda vinculada pero 0 usuarios, lanzamos el modal
        if (globalState.vaultPath) {
          isModalOpen = true;
        }
      }
    } catch (e) { 
        if (e.message !== "Unauthorized") console.error("[AUTH] Error en loadUsers:", e); 
    } finally {
      isLoading = false;
      globalState.isSyncing = false;
    }
  }

  async function promptForVaultPath(mode = 'open') {
    if (invoke) {
      try {
        const invoker = await invoke();
        const path = await invoker('browse_file', { mode });
        if (path) {
          await api.post('/vault/config/path', { path, force_init: mode === 'save' });
          globalState.vaultPath = path;
          loadUsers(true);
        }
      } catch (e) {
        console.error('promptForVaultPath error:', e);
        globalState.notify('Error', `No se pudo abrir la bóveda: ${e.message || e}`, 'error');
      }
    } else {
      if (mode === 'open') {
        const input = document.createElement('input');
        input.type = 'file';
        input.accept = '.pcty';
        input.onchange = async (e) => {
          const file = e.target.files[0];
          if (file) {
            const formData = new FormData();
            formData.append('file', file);
            await api.post('/vault/upload', formData);
            loadUsers(true);
          }
        };
        input.click();
      } else {
        isNamingVault = true;
      }
    }
  }

  function handleNewUserClick() {
    isModalOpen = true;
    isDropdownOpen = false;
    pillRect = null;
  }

  function toggleDropdown() {
    isDropdownOpen = !isDropdownOpen;
    if (!isDropdownOpen) {
      pillRect = null;
    } else {
      tick().then(() => {
        if (pillRef) {
          const rect = pillRef.getBoundingClientRect();
          pillRect = { top: rect.bottom + 8, left: rect.left, width: rect.width };
        }
      });
    }
  }

  async function handleLock() {
    try {
      await api.post('/auth/lock', {}, { silent: true });
    } catch (e) {
      // Ignoramos error si la sesión ya expiró o devuelve 400
    } finally {
      isUnlocked = false;
      showManager = false;
      globalState.resetSessionState();
    }
  }

  function handleUserSelect(user) {
    if (isUnlocked) return;
    selectedUser = user;
    globalState.selectedUser = user;
    globalState.userColor = user.color;
    isDropdownOpen = false;
    pillRect = null;
  }

  function handleAddUser() { loadUsers(); }

  function openEntryModal(mode = 'normal') {
    entryToEdit = null;
    entryModalMode = mode;
    isEntryModalOpen = true;
  }

  function handleEditEntry(entry) {
    entryToEdit = entry;
    entryModalMode = 'normal';
    isEntryModalOpen = true;
  }

  // Cifra/descifra ya se resuelve en el modal; aquí persistimos en el backend.
  async function handleSaveEntry(payload) {
    const { id, ...body } = payload;
    try {
      if (id) {
        await api.put(`/vault/accounts/${id}`, body);
        globalState.notify('Bóveda', 'Entrada actualizada', 'success');
      } else {
        await api.post('/vault/accounts', body);
        globalState.notify('Bóveda', 'Entrada guardada', 'success');
      }
      if (vaultRef) await vaultRef.loadAccounts();
      globalState.refreshTrashCount();
      return true;
    } catch (e) {
      // api.js ya notifica el error; mantenemos el modal abierto.
      return false;
    }
  }

  function handleDatabaseChange() {
    loadFolders();
    globalState.refreshTrashCount();
  }

  // Drag & Drop
  function handleDragOver(e) { e.preventDefault(); if (!isUnlocked) isDragging = true; }
  function handleDragLeave() { isDragging = false; }
  async function handleDrop(e) {
    e.preventDefault();
    isDragging = false;
    const file = e.dataTransfer.files[0];
    if (file && file.name.endsWith('.pcty')) {
      const formData = new FormData();
      formData.append('file', file);
      await api.post('/vault/upload', formData);
      loadUsers(true);
    }
  }

  onMount(() => {
    globalState.initialize().then(() => loadUsers());

    const onRefresh = () => loadUsers();
    window.addEventListener('pact:refresh-users', onRefresh);
    window.addEventListener('pact:force-lock', handleLock);

    return () => {
      window.removeEventListener('pact:refresh-users', onRefresh);
      window.removeEventListener('pact:force-lock', handleLock);
    };
  });

  // Auto-bloqueo por inactividad y auto-sync mientras la bóveda está abierta.
  // `untrack` evita que los cambios de estado internos (isSyncing, timers...)
  // vuelvan a disparar este efecto en bucle.
  $effect(() => {
    const unlocked = isUnlocked;
    const mode = globalState.dbMode;
    untrack(() => {
      if (unlocked) {
        globalState.startInactivityWatch();
        if (mode === 'remote' || mode === 'hybrid') {
          globalState.startAutoSync();
        } else {
          globalState.stopAutoSync();
        }
      } else {
        globalState.stopInactivityWatch();
        globalState.stopAutoSync();
      }
    });
  });

  function handleKeydown(e) {
    if (e.key === 'Escape') {
      if (isEntryModalOpen) { isEntryModalOpen = false; return; }
      if (isSettingsOpen) { isSettingsOpen = false; return; }
      if (isModalOpen) { isModalOpen = false; return; }
    }
    if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
      e.preventDefault();
      const searchInput = document.querySelector('input[type="text"][placeholder*="buscar" i], input[type="text"][placeholder*="Buscar" i], input[placeholder*="Filtrar"]');
      searchInput?.focus();
    }
    if ((e.ctrlKey || e.metaKey) && e.key === 'l') {
      e.preventDefault();
      handleLock();
    }
    if ((e.ctrlKey || e.metaKey) && e.key === 'n') {
      e.preventDefault();
      if (isUnlocked) openEntryModal('normal');
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<main 
  ondragover={handleDragOver}
  ondragleave={handleDragLeave}
  ondrop={handleDrop}
  class="h-screen bg-[var(--md-background)] text-[var(--md-on-background)] font-sans flex flex-col relative transition-colors duration-700 overflow-hidden { !isUnlocked ? 'items-center justify-center' : '' }">
  
  {#if isDragging}
    <div class="fixed inset-0 z-[100] bg-[var(--md-primary)]/20 backdrop-blur-xl flex items-center justify-center animate-in fade-in duration-300 pointer-events-none border-[6px] border-dashed border-[var(--md-primary)]/40 m-8 rounded-[64px]">
      <div class="flex flex-col items-center gap-6 scale-110">
        <div class="w-32 h-32 rounded-[48px] bg-[var(--md-primary)] text-[var(--md-on-primary)] flex items-center justify-center shadow-elev-2  animate-bounce">
          <File size={64} strokeWidth={2.5} />
        </div>
        <div class="text-center">
          <h2 class="text-4xl font-black tracking-tight text-[var(--md-primary)]">{i18n.t('app.dropVault')}</h2>
          <p class="text-[12px] font-black uppercase tracking-[0.3em] opacity-60 mt-2 text-[var(--md-primary)]">{i18n.t('app.dropVaultHint')}</p>
        </div>
      </div>
    </div>
  {/if}

  <NotificationProvider />
  
  {#if isDropdownOpen && !isUnlocked}
    <!-- Dropdown Click-Outside Backdrop moved to TOP level -->
    <div 
      class="fixed inset-0 z-[60] bg-black/10 lg:hidden cursor-default"
      onclick={() => { isDropdownOpen = false; pillRect = null; }}
      onkeydown={(e) => e.key === 'Escape' && (isDropdownOpen = false)}
      role="button"
      tabindex="-1"
    ></div>
    
    <!-- Mobile Dropdown (positioned fixed para estar sobre el backdrop) -->
    {#if pillRect}
      <div in:fly={{ y: -10, duration: 300 }} out:fade={{ duration: 200 }}
           class="fixed z-[61] p-4 bg-[var(--md-surface-2)] rounded-[32px] ring-window shadow-elev-3 max-h-[300px] overflow-y-auto custom-scrollbar lg:hidden"
           style="top: {pillRect.top}px; left: {pillRect.left}px; width: {pillRect.width}px">
        <UserList onSelect={(u) => { handleUserSelect(u); }} selectedId={selectedUser?.id} {isMobile} bind:users={users} onAddClick={handleNewUserClick} />
      </div>
    {/if}
  {/if}
  
  {#if !isUnlocked || (isUnlocked && !showManager)}
    <div class="fixed top-8 right-8 flex gap-3 z-50">
      <button 
        onclick={() => isSettingsOpen = true}
        class="p-3 rounded-2xl bg-[var(--md-surface)]/60 backdrop-blur-md border border-border opacity-40 hover:opacity-100 hover:scale-110 active:scale-95 transition-all"
        title={i18n.t('app.settings')}
        aria-label={i18n.t('app.settings')}
      >
        <Settings size={20} strokeWidth={2.5} />
      </button>
    </div>
  {/if}

  <div class="flex-1 w-full {isUnlocked ? 'max-w-none' : 'max-w-7xl'} mx-auto relative lg:overflow-hidden transition-all duration-700 ease-in-out">
    <div class="grid {isUnlocked || users.length === 0 ? 'grid-cols-1' : 'grid-cols-1 lg:grid-cols-[1fr_400px_1fr]'} w-full h-full gap-12 px-4 lg:p-8 transition-all duration-700 ease-in-out">

      {#if !isUnlocked && users.length > 0}
        <div class="hidden lg:block h-full"></div>
      {/if}

      <div class="flex flex-col items-center justify-center h-full w-full z-10 transition-all duration-700 ease-in-out">
        {#if selectedUser && !isUnlocked}
          
          <!-- Mobile User Profile Pill -->
          <div class="w-full max-w-[400px] mb-6 flex lg:hidden flex-col gap-2">
            <div class="w-full flex items-center bg-[var(--md-surface-2)] h-20 rounded-full border border-border group/pill">
              <button 
                bind:this={pillRef}
                onclick={toggleDropdown}
                class="flex-1 h-full px-3 flex items-center rounded-l-full active:scale-[0.98] transition-all min-w-0 cursor-pointer"
              >
                <div class="w-14 h-14 rounded-full overflow-hidden shrink-0 bg-black/5 p-0.5 border-2 ml-1" style="border-color: {selectedUser.color}66">
                  <img src={selectedUser.avatar} alt="avatar" class="w-full h-full object-cover rounded-full" />
                </div>
                <div class="flex flex-col text-left flex-grow ml-4 pr-4 min-w-0">
                  <span class="text-sm font-bold truncate text-[var(--md-on-surface)]">{selectedUser.name}</span>
                  <span class="text-[10px] font-black uppercase tracking-widest text-[var(--md-on-surface-variant)]">
                    {isDropdownOpen ? i18n.t('common.close') : i18n.t('app.change')}
                  </span>
                </div>
                <ChevronDown size={20} class="opacity-50 transition-transform duration-300 mr-2 {isDropdownOpen ? 'rotate-180' : ''}" />
              </button>
            </div>
          </div>
          
          <div class="w-full max-w-[400px] relative z-10">
            <LoginBox user={selectedUser} onUnlock={() => { isUnlocked = true; loadFolders(); }} />
          </div>
        {:else if isUnlocked}
          <div class="w-full h-full animate-in fade-in slide-in-from-bottom-4 duration-700">
            <VaultView 
              bind:this={vaultRef}
              user={selectedUser} 
              {folders}
              onLock={handleLock} 
              onOpenSettings={() => isSettingsOpen = true}
              onOpenNewEntry={() => openEntryModal('normal')}
              onOpenNewFolder={() => isNamingFolder = true}
              onEditEntry={handleEditEntry}
              onDatabaseChange={handleDatabaseChange}
              bind:showManager
            />
          </div>
        {:else if !isLoading}
          <!-- PANTALLA DE INICIO SUBTIL -->
          <div class="flex flex-col items-center text-center opacity-50 gap-8 animate-in fade-in zoom-in duration-700">
            <div class="flex flex-col gap-2">
              {#if globalState.vaultPath}
                <p class="text-xl lg:text-2xl font-bold tracking-tight">{i18n.t('app.emptyVaultTitle', { name: globalState.vaultPath.split('/').pop() })}</p>
                <p class="text-sm opacity-60">{i18n.t('app.emptyVaultHint')}</p>
              {:else}
                <p class="text-xl lg:text-2xl font-bold tracking-tight">{i18n.t('app.noUsersTitle')}</p>
              {/if}
            </div>
            
            <div class="flex flex-col gap-4 p-6 border-2 border-dashed border-border rounded-[48px] bg-black/[0.02] dark:bg-white/[0.01]">
              <button onclick={() => promptForVaultPath('open')} class="flex items-center gap-4 p-4 pr-12 transition-all duration-300 bg-[var(--md-surface)] border border-border rounded-full hover:opacity-100 hover:border-[var(--md-primary)]/40 hover:bg-[var(--md-primary)]/5 active:scale-95 group w-full">
                <div class="w-12 h-12 rounded-full bg-[var(--md-primary)] text-[var(--md-on-primary)] flex items-center justify-center shrink-0 shadow-elev-1 group-hover:scale-110 transition-transform"><File size={24} /></div>
                <div class="flex flex-col text-left">
                  <span class="text-[10px] font-black uppercase tracking-[0.2em]">{i18n.t('app.openVault')}</span>
                  <span class="text-[9px] text-[var(--md-on-surface-variant)] font-bold uppercase tracking-widest">{i18n.t('app.openVaultHint')}</span>
                </div>
              </button>

              <div class="h-px w-12 bg-black/10 dark:bg-white/10 mx-auto"></div>

              <button onclick={() => promptForVaultPath('save')} class="flex items-center gap-4 p-4 pr-12 transition-all duration-300 bg-[var(--md-surface)] border border-border rounded-full hover:opacity-100 hover:border-[var(--md-primary)]/40 hover:bg-[var(--md-primary)]/5 active:scale-95 group w-full">
                <div class="w-12 h-12 rounded-full bg-[var(--md-primary)]/10 text-[var(--md-primary)] flex items-center justify-center shrink-0 border border-[var(--md-primary)]/20 group-hover:scale-110 transition-transform"><Plus size={24} /></div>
                <div class="flex flex-col text-left">
                  <span class="text-[10px] font-black uppercase tracking-[0.2em]">{i18n.t('app.createVault')}</span>
                  <span class="text-[9px] text-[var(--md-on-surface-variant)] font-bold uppercase tracking-widest">{i18n.t('app.createVaultHint')}</span>
                </div>
              </button>
            </div>
          </div>
        {/if}
      </div>

      {#if !isUnlocked && users.length > 0}
        <div class="hidden lg:flex flex-col justify-center h-full animate-in fade-in slide-in-from-right duration-1000">
          <div class="p-2">
            <h2 class="text-[10px] font-black uppercase tracking-[0.2em] text-[var(--md-on-surface-variant)] mb-6 ml-4">{i18n.t('app.users')}</h2>
            <UserList onSelect={handleUserSelect} selectedId={selectedUser?.id} {isMobile} bind:users={users} onAddClick={handleNewUserClick} />
          </div>
        </div>
      {/if}
    </div>
  </div>

  <div class="w-full py-4 bg-transparent animate-in fade-in duration-1000 shrink-0">
    <Footer />
  </div>

  <!-- MODALS -->
  <NewUserModal isOpen={isModalOpen} onSave={handleAddUser} onClose={() => isModalOpen = false} {users} />
  <SettingsModal isOpen={isSettingsOpen} isUnlocked={isUnlocked} {folders} onRefreshFolders={loadFolders} onClose={() => isSettingsOpen = false} bind:users={users} />
  
  <NewEntryModal 
    isOpen={isEntryModalOpen} 
    mode={entryModalMode} 
    entry={entryToEdit}
    {folders} 
    user={selectedUser}
    onSave={handleSaveEntry}
    onOpenNewFolder={() => isNamingFolder = true}
    onClose={() => { isEntryModalOpen = false; entryToEdit = null; }} 
  />

  <FolderModal 
    bind:isOpen={isNamingFolder} 
    userColor={selectedUser?.color} 
    onFolderCreated={() => { loadFolders(); if (vaultRef) vaultRef.loadAccounts(); }} 
  />

  <VaultModal 
    bind:isOpen={isNamingVault} 
    onVaultCreated={() => loadUsers(true)} 
  />

</main>

<style>
  :global(body) { margin: 0; padding: 0; overflow: hidden; }
  @keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }
  @keyframes zoom-in { from { transform: scale(0.95); } to { transform: scale(1); } }
  @keyframes slide-in-from-right { from { transform: translateX(20px); opacity: 0; } to { transform: translateX(0); opacity: 1; } }
  @keyframes slide-in-from-bottom { from { transform: translateY(20px); opacity: 0; } to { transform: translateY(0); opacity: 1; } }
  .animate-in { animation-fill-mode: both; }
  .fade-in { animation-name: fade-in; }
  .zoom-in { animation-name: zoom-in; }
  .slide-in-from-right { animation-name: slide-in-from-right; }
  .slide-in-from-bottom-4 { animation-name: slide-in-from-bottom; }
</style>
