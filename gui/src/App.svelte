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
  import { onMount } from 'svelte';
  import { fly, fade } from 'svelte/transition';
  import { globalState } from './lib/state.svelte.js';
  import { api } from './lib/api.js';
  import { Settings, ChevronDown, Plus, File, Sun, Moon } from 'lucide-svelte';
  
  // Soporte para Tauri v2
  const invoke = typeof window !== 'undefined' && (window.__TAURI__ || !!window.__TAURI_INTERNALS__) 
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
  let folders = $state([]);
  let vaultRef = $state();
  let isUnlocked = $state(false);
  let showManager = $state(false);
  let isDragging = $state(false);

  // Flags de Modals
  let isNamingFolder = $state(false);
  let isNamingVault = $state(false);

  async function loadFolders() {
    try {
      folders = await api.get('/vault/folders');
    } catch (e) { /* Error manejado por api.js */ }
  }

  async function loadVaultPath() {
    try {
      const data = await api.get('/vault/config/path', { silent: true });
      globalState.vaultPath = data.path || '';
    } catch (e) { console.error(e); }
  }

  async function loadUsers(showToast = false) {
    console.log("[DEBUG] Iniciando loadUsers...");
    globalState.isSyncing = true;
    users = [];
    await loadVaultPath();
    console.log("[DEBUG] Vault Path tras loadVaultPath:", globalState.vaultPath);
    try {
      const data = await api.get('/auth/users');
      console.log("[DEBUG] Respuesta /auth/users:", data);
      const userList = data.users || [];
      if (data.source) globalState.dbMode = data.source;
      globalState.vaultPath = data.vault_path || '';

      users = userList.map(u => ({
        id: u.id,
        name: u.username, // El alias elegido por el usuario
        username: u.username,
        status: u.status,
        color: u.color || '#6750A4',
        avatar: `https://api.dicebear.com/9.x/notionists/svg?seed=${u.username}`
      }));
      console.log("[DEBUG] Usuarios mapeados:", users.length);

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
        console.log("[DEBUG] Usuario seleccionado automáticamente:", selectedUser.name);
      } else {
        selectedUser = null;
        globalState.selectedUser = null;
        console.log("[DEBUG] No se encontraron usuarios.");
        
        // Si hay boveda vinculada pero 0 usuarios, lanzamos el modal
        if (globalState.vaultPath) {
          console.log("[DEBUG] Bóveda vacía, abriendo creador de usuarios.");
          isModalOpen = true;
        }
      }
    } catch (e) { 
        if (e.message !== "Unauthorized") console.error("[DEBUG] Error en loadUsers:", e); 
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
      } catch (e) { console.error(e); }
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

  async function handleNewUserClick() {
    if (!globalState.vaultPath) {
      promptForVaultPath('open');
      return;
    }
    isModalOpen = true;
  }

  async function handleLock() {
    await api.post('/auth/lock', {}, { silent: true });
    isUnlocked = false;
    showManager = false;
    globalState.resetSessionState();
  }

  function handleUserSelect(user) {
    if (isUnlocked) return;
    selectedUser = user;
    globalState.selectedUser = user;
    globalState.userColor = user.color;
    isDropdownOpen = false;
  }

  function handleAddUser() { loadUsers(); }

  function openEntryModal(mode) {
    entryModalMode = mode;
    isEntryModalOpen = true;
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
    const interval = setInterval(() => { if (isUnlocked) globalState.refreshPendingCount(); }, 30000);
    
    window.addEventListener('pact:refresh-users', () => loadUsers());
    window.addEventListener('pact:force-lock', handleLock);

    return () => {
      clearInterval(interval);
      window.removeEventListener('pact:refresh-users', () => loadUsers());
      window.removeEventListener('pact:force-lock', handleLock);
    };
  });
</script>

<main 
  ondragover={handleDragOver}
  ondragleave={handleDragLeave}
  ondrop={handleDrop}
  class="h-screen bg-[#f6f8fb] dark:bg-[#0b0a0d] text-[#1c1b1f] dark:text-[#e6e1e5] font-sans flex flex-col relative transition-colors duration-700 overflow-hidden { !isUnlocked ? 'items-center justify-center' : '' }">
  
  {#if isDragging}
    <div class="fixed inset-0 z-[100] bg-[#6750A4]/20 backdrop-blur-xl flex items-center justify-center animate-in fade-in duration-300 pointer-events-none border-[6px] border-dashed border-[#6750A4]/40 m-8 rounded-[64px]">
      <div class="flex flex-col items-center gap-6 scale-110">
        <div class="w-32 h-32 rounded-[48px] bg-[#6750A4] text-white flex items-center justify-center shadow-2xl shadow-[#6750A4]/40 animate-bounce">
          <File size={64} strokeWidth={2.5} />
        </div>
        <div class="text-center">
          <h2 class="text-4xl font-black tracking-tight text-[#6750A4]">Suelta tu Bóveda</h2>
          <p class="text-[12px] font-black uppercase tracking-[0.3em] opacity-60 mt-2 text-[#6750A4]">Solo archivos .pcty</p>
        </div>
      </div>
    </div>
  {/if}

  <NotificationProvider />
  
  {#if isDropdownOpen && !isUnlocked}
    <!-- Dropdown Click-Outside Backdrop moved to TOP level -->
    <div 
      class="fixed inset-0 z-[60] bg-black/10 lg:hidden cursor-default"
      onclick={() => { console.log("[DEBUG] Backdrop click -> closing dropdown"); isDropdownOpen = false; }}
      onkeydown={(e) => e.key === 'Escape' && (isDropdownOpen = false)}
      role="button"
      tabindex="-1"
    ></div>
  {/if}
  
  {#if !isUnlocked || (isUnlocked && !showManager)}
    <div class="fixed top-8 right-8 flex gap-3 z-50">
      <button 
        onclick={() => isSettingsOpen = true}
        class="p-3 rounded-2xl bg-white/50 dark:bg-white/5 backdrop-blur-md border border-black/[0.08] dark:border-white/[0.08] opacity-40 hover:opacity-100 hover:scale-110 active:scale-95 transition-all"
        title="Configuración"
      >
        <Settings size={20} strokeWidth={2.5} />
      </button>
    </div>
  {/if}

  <div class="flex-1 w-full {isUnlocked ? 'max-w-none' : 'max-w-7xl'} mx-auto relative overflow-hidden transition-all duration-700 ease-in-out">
    <div class="grid {isUnlocked || users.length === 0 ? 'grid-cols-1' : 'grid-cols-1 lg:grid-cols-[1fr_400px_1fr]'} w-full h-full gap-12 px-4 lg:p-8 transition-all duration-700 ease-in-out">

      {#if !isUnlocked && users.length > 0}
        <div class="hidden lg:block h-full"></div>
      {/if}

      <div class="flex flex-col items-center justify-center h-full w-full z-10 transition-all duration-700 ease-in-out">
        {#if selectedUser && !isUnlocked}
          
          <!-- Mobile User Profile Pill -->
          <div class="w-full max-w-[400px] mb-8 relative z-[70] flex lg:hidden flex-col gap-2">
            <div class="w-full flex items-center bg-white dark:bg-[#16151a] h-20 rounded-full border border-black/[0.08] dark:border-white/[0.08] group/pill">
              <button 
                onclick={() => { 
                  console.log("[DEBUG] Toggle dropdown click. Current state:", isDropdownOpen);
                  isDropdownOpen = !isDropdownOpen; 
                }}
                class="flex-1 h-full px-3 flex items-center rounded-l-full active:scale-[0.98] transition-all min-w-0 cursor-pointer"
              >
                <div class="w-14 h-14 rounded-full overflow-hidden shrink-0 bg-black/5 p-0.5 border-2 ml-1" style="border-color: {selectedUser.color}66">
                  <img src={selectedUser.avatar} alt="avatar" class="w-full h-full object-cover rounded-full" />
                </div>
                <div class="flex flex-col text-left flex-grow ml-4 pr-4 min-w-0">
                  <span class="text-sm font-bold truncate">{selectedUser.name}</span>
                  <span class="text-[10px] font-black uppercase tracking-widest opacity-30">
                    {isDropdownOpen ? 'Cerrar' : 'Cambiar / Nuevo'}
                  </span>
                </div>
                <ChevronDown size={20} class="opacity-30 transition-transform duration-300 mr-2 {isDropdownOpen ? 'rotate-180' : ''}" />
              </button>
            </div>

            <!-- Actual Dropdown Content -->
            {#if isDropdownOpen}
              <div in:fly={{ y: -10, duration: 300 }} out:fade={{ duration: 200 }} 
                   class="absolute top-full left-0 right-0 mt-2 p-4 bg-white dark:bg-[#16151a] rounded-[32px] border border-black/[0.08] dark:border-white/[0.08] shadow-2xl z-[80] max-h-[300px] overflow-y-auto custom-scrollbar">
                <UserList onSelect={(u) => { console.log("[DEBUG] User selected from dropdown:", u.name); handleUserSelect(u); }} selectedId={selectedUser?.id} {isMobile} bind:users={users} onAddClick={handleNewUserClick} />
              </div>
            {/if}
          </div>
          
          <div class="w-full max-w-[400px] relative z-10 rounded-[32px] animate-in fade-in zoom-in duration-500 border border-black/[0.05] dark:border-white/[0.05]">
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
              onOpenNewEntry={(m) => openEntryModal(m || 'normal')}
              onOpenNewFolder={() => isNamingFolder = true}
              bind:showManager
            />
          </div>
        {:else if !isLoading}
          <!-- PANTALLA DE INICIO SUBTIL -->
          <div class="flex flex-col items-center text-center opacity-50 gap-8 animate-in fade-in zoom-in duration-700">
            <div class="flex flex-col gap-2">
              {#if globalState.vaultPath}
                <p class="text-xl lg:text-2xl font-bold tracking-tight">Bóveda vacía: {globalState.vaultPath.split('/').pop()}</p>
                <p class="text-sm opacity-60">Crea tu primer usuario en esta bóveda para continuar.</p>
              {:else}
                <p class="text-xl lg:text-2xl font-bold tracking-tight">Crea tu primer usuario para empezar.</p>
              {/if}
            </div>
            
            <div class="flex flex-col gap-4 p-6 border-2 border-dashed border-black/20 dark:border-white/20 rounded-[48px] bg-black/[0.02] dark:bg-white/[0.01]">
              <button onclick={() => promptForVaultPath('open')} class="flex items-center gap-4 p-4 pr-12 transition-all duration-300 bg-white dark:bg-white/5 border border-black/10 dark:border-white/10 rounded-full hover:opacity-100 hover:border-[#6750A4]/50 hover:bg-[#6750A4]/5 active:scale-95 group w-full">
                <div class="w-12 h-12 rounded-full bg-[#6750A4] text-white flex items-center justify-center shrink-0 shadow-sm group-hover:scale-110 transition-transform"><File size={24} /></div>
                <div class="flex flex-col text-left">
                  <span class="text-[10px] font-black uppercase tracking-[0.2em]">Abrir Bóveda</span>
                  <span class="text-[9px] opacity-40 font-bold uppercase tracking-widest">Seleccionar archivo .pcty</span>
                </div>
              </button>

              <div class="h-px w-12 bg-black/10 dark:bg-white/10 mx-auto"></div>

              <button onclick={() => promptForVaultPath('save')} class="flex items-center gap-4 p-4 pr-12 transition-all duration-300 bg-white dark:bg-white/5 border border-black/10 dark:border-white/10 rounded-full hover:opacity-100 hover:border-[#6750A4]/50 hover:bg-[#6750A4]/5 active:scale-95 group w-full">
                <div class="w-12 h-12 rounded-full bg-[#6750A4]/10 text-[#6750A4] flex items-center justify-center shrink-0 border border-[#6750A4]/20 group-hover:scale-110 transition-transform"><Plus size={24} /></div>
                <div class="flex flex-col text-left">
                  <span class="text-[10px] font-black uppercase tracking-[0.2em]">Crear Nueva</span>
                  <span class="text-[9px] opacity-40 font-bold uppercase tracking-widest">Empezar desde cero</span>
                </div>
              </button>
            </div>
          </div>
        {/if}
      </div>

      {#if !isUnlocked && users.length > 0}
        <div class="hidden lg:flex flex-col justify-center h-full animate-in fade-in slide-in-from-right duration-1000">
          <div class="p-2">
            <h2 class="text-[10px] font-black uppercase tracking-[0.2em] opacity-30 mb-6 ml-4">Usuarios</h2>
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
    {folders} 
    user={selectedUser}
    onOpenNewFolder={() => isNamingFolder = true}
    onClose={() => isEntryModalOpen = false} 
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
  :global(body) { margin: 0; padding: 0; background-color: #f6f8fb; overflow: hidden; }
  :global(.dark body) { background-color: #0b0a0d; }
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
