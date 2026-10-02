<script>
  import { LogOut, Search, Plus, Folder, Settings, Bell, Star, Shield, ArrowLeft, ChevronDown, Trash2, Ghost, Gamepad2, Tv, Music, Coffee, Heart, ShoppingBag, Luggage, Wallet, Umbrella, X } from 'lucide-svelte';
  import { onMount } from 'svelte';
  
  import Sidebar from './vault/Sidebar.svelte';
  import AccountCard from './vault/AccountCard.svelte';
  import { globalState } from './state.svelte.js';
  import { api } from './api.js';
  import { decryptE2E, encryptE2E, generateTOTP } from './crypto_utils.js';
  import brandWhitelist from './brand_whitelist.json';

  let { user, folders, onLock, onOpenSettings, onOpenNewEntry, onOpenNewFolder, onEditEntry, onDatabaseChange, showManager = $bindable() } = $props();
  
  // Mapa de iconos para renderizado dinámico
  const iconMap = {
    'folder': Folder,
    'shield': Shield,
    'lock': Folder, // Fallback
    'ghost': Ghost,
    'gamepad2': Gamepad2,
    'tv': Tv,
    'music': Music,
    'coffee': Coffee,
    'heart': Heart,
    'star': Star,
    'shopping-bag': ShoppingBag,
    'luggage': Luggage,
    'wallet': Wallet,
    'umbrella': Umbrella,
    'trash-2': Trash2
  };
  
  let items = $state([]);
  let isLoading = $state(true);
  let activeFilter = $state('all'); 
  let loadedIcons = $state({});
  let isInboxOpen = $state(false);
  let inboxItems = $state([]);
  let isInboxLoading = $state(false);

  async function openInbox() {
    isInboxOpen = true;
    isInboxLoading = true;
    try {
      const raw = await api.get('/vault/pending-entries', { silent: true });
      inboxItems = await Promise.all((raw || []).map(async (item) => {
        let label = `Entrada pendiente #${item.id}`;
        if (globalState.masterKey && item.data_blob) {
          const data = await decryptE2E(item.data_blob, globalState.masterKey);
          if (typeof data === 'string' && data) label = data;
          else if (data && typeof data === 'object') label = data.title || data.username_site || label;
        }
        return { id: item.id, label, username: item.username };
      }));
    } catch (e) {
      inboxItems = [];
    } finally {
      isInboxLoading = false;
    }
  }

  async function deleteInboxItem(id) {
    try {
      await api.delete(`/vault/pending-entries/${id}`);
      inboxItems = inboxItems.filter(i => i.id !== id);
      globalState.pendingCount = Math.max(0, globalState.pendingCount - 1);
      globalState.notify('Bandeja', 'Elemento descartado.', 'success');
    } catch (e) { /* api.js ya notifica */ }
  }

  // --- LÓGICA DE NOTIFICACIONES ---
  let totalNotifications = $derived.by(() => {
    let count = globalState.pendingCount;
    if (globalState.trashCount > 10) count += 1;
    const isSyncStale = Date.now() - globalState.lastSyncSuccess > 30 * 60 * 1000;
    if (globalState.syncError || isSyncStale) count += 1;
    return count;
  });

  // --- LÓGICA DE CARGA ---

  export async function loadAccounts() {
    globalState.isSyncing = true;
    try {
      if (!globalState.masterKey) return;

      const includeDeleted = activeFilter === 'trash';
      let url = `/vault/accounts?include_deleted=${includeDeleted}`;
      
      if (activeFilter === 'favorites') {
        url += `&is_favorite=true`;
      } else if (activeFilter.startsWith('folder-')) {
        const folderId = activeFilter.split('-')[1];
        url += `&folder_id=${folderId}`;
      }

      const data = await api.get(url);
      
      const decryptedItems = await Promise.all(data.map(async (acc) => {
        const folder = folders.find(f => f.id === acc.folder_id);
        const dTitle = await decryptE2E(acc.title, globalState.masterKey);
        const dUrl = acc.url ? await decryptE2E(acc.url, globalState.masterKey) : '';
        const dUser = acc.username_site ? await decryptE2E(acc.username_site, globalState.masterKey) : '';
        const dPass = await decryptE2E(acc.encrypted_pass, globalState.masterKey);
        // El icono se guarda cifrado E2E; si falla, asumimos formato antiguo.
        const dIcon = acc.icon_data ? await decryptE2E(acc.icon_data, globalState.masterKey) : null;

        return {
          id: acc.id,
          title: dTitle || 'Contenido Cifrado', 
          username: dUser || 'Sin usuario',
          password: dPass || '••••••••',
          website: dUrl || '',
          strength: 'Segura',
          icon: (dTitle || '??').substring(0, 2).toUpperCase(),
          color: '#6750A4',
          folder_id: acc.folder_id,
          folder_icon: folder ? folder.icon : 'folder',
          is_favorite: acc.is_favorite,
          is_deleted: !!acc.deleted_at,
          icon_data: dIcon || acc.icon_data,
          has_totp: acc.has_totp,
          totp: null 
        };
      }));

      items = decryptedItems;
      updateAllTOTP();

    } catch (error) {
      console.error('Error cargando datos:', error);
    } finally {
      isLoading = false;
      globalState.isSyncing = false;
    }
  }

  async function toggleFavorite(id) {
    try {
      const data = await api.patch(`/vault/accounts/${id}/favorite`);
      items = items.map(item => item.id === id ? {...item, is_favorite: data.is_favorite} : item);
      globalState.notify('Favoritos', data.is_favorite ? 'Añadido' : 'Eliminado', 'success', 2000);
      onDatabaseChange?.();
    } catch (e) {
      if (e.message !== 'Unauthorized') console.warn('Error al marcar favorito:', e);
    }
  }

  function editAccount(entry) {
    onEditEntry?.(entry);
  }

  async function deleteAccount(id) {
    if (!confirm('¿Mover esta cuenta a la papelera?')) return;
    try {
      await api.delete(`/vault/accounts/${id}`);
      globalState.notify('Papelera', 'Cuenta movida a la papelera', 'success');
      expandedCardId = null;
      await loadAccounts();
      globalState.refreshTrashCount();
      onDatabaseChange?.();
    } catch (e) {
      if (e.message !== 'Unauthorized') console.warn('Error al eliminar cuenta:', e);
    }
  }

  async function restoreAccount(id) {
    try {
      await api.post(`/vault/accounts/${id}/restore`);
      globalState.notify('Papelera', 'Cuenta restaurada', 'success');
      loadAccounts();
      onDatabaseChange?.();
    } catch (e) {
      if (e.message !== 'Unauthorized') console.warn('Error al restaurar:', e);
    }
  }

  async function handleEmptyTrash() {
    if (!confirm('¿Seguro que quieres vaciar la papelera?')) return;
    try {
      const data = await api.delete('/vault/accounts/trash/empty');
      globalState.notify('Papelera', `Se han borrado ${data.deleted_count} cuentas.`, 'success');
      loadAccounts();
      onDatabaseChange?.();
    } catch (e) {
      if (e.message !== 'Unauthorized') console.warn('Error al vaciar papelera:', e);
    }
  }

  // --- LÓGICA TOTP ---
  let totpProgress = $state(100);
  let totpSeconds = $state(30);
  let totpInterval = $state(null);

  async function fetchTOTP(accountId) {
    try {
      // El servidor devuelve el seed cifrado; lo desciframos y generamos el
      // código en local (Zero-Knowledge).
      const data = await api.get(`/vault/totp/${accountId}`, { silent: true });
      if (!data?.seed || !globalState.masterKey) return;
      const seed = await decryptE2E(data.seed, globalState.masterKey);
      if (!seed) return;
      const code = await generateTOTP(String(seed).trim());
      items = items.map(item => item.id === accountId ? {...item, totp: code} : item);
    } catch (e) { /* seed ilegible o TOTP inválido */ }
  }

  async function updateAllTOTP() {
    const accountsWithTOTP = items.filter(i => i.has_totp);
    await Promise.all(accountsWithTOTP.map(acc => fetchTOTP(acc.id)));
  }

  function startTOTPTimer() {
    if (totpInterval) clearInterval(totpInterval);
    totpInterval = setInterval(() => {
      if (totpSeconds > 0) {
        totpSeconds--;
        totpProgress = (totpSeconds / 30) * 100;
      } else {
        totpSeconds = 30;
        totpProgress = 100;
        updateAllTOTP();
      }
    }, 1000);
  }

  // --- ICONOS ---
  const brandSet = new Set(brandWhitelist);

  function getIconUrl(item, attempt = 0) {
    if (!item || item.website === 'none') return null;
    let domain = '';
    const website = (item.website || '').trim();
    const title = (item.title || '').trim();

    if (website && website.length > 2) {
      try {
        const urlStr = website.startsWith('http') ? website : `https://${website}`;
        const url = new URL(urlStr);
        domain = url.hostname.replace('www.', '').toLowerCase();
      } catch (e) { domain = website.replace('www.', '').split('/')[0].toLowerCase(); }
    } else {
      if (!title || title.includes(' ') || title.length > 25 || title.length < 2) return null;
      const titleClean = title.toLowerCase();
      const possibleDomains = [`${titleClean}.com`, `${titleClean}.org`, `${titleClean}.io`, titleClean];
      domain = possibleDomains.find(d => brandSet.has(d));
      if (!domain) return null;
    }

    if (!domain || domain.length < 4 || !domain.includes('.') || domain.includes(' ')) return null;
    if (attempt === 0) return `https://unavatar.io/${domain}?fallback=false`;
    if (attempt === 1) return `https://logo.hunter.io/${domain}`;
    if (attempt === 2) return `https://www.google.com/s2/favicons?sz=128&domain=${domain}`;
    return null;
  }

  async function saveIconToDB(id, url) {
    try {
      const data = await api.get(`/vault/proxy-icon?url=${encodeURIComponent(url)}`, { silent: true });
      if (!data.base64 || !globalState.masterKey) return;
      // El icono se cifra E2E antes de persistirlo (no debe revelar el servicio).
      const encryptedIcon = await encryptE2E(data.base64, globalState.masterKey);
      await api.patch(`/vault/accounts/${id}/icon`, { icon_data: encryptedIcon });
      items = items.map(it => it.id === id ? {...it, icon_data: data.base64} : it);
    } catch (e) {}
  }

  function handleIconLoad(id, url, e) {
    e.target.classList.remove('opacity-0');
    loadedIcons[id] = true;
    const item = items.find(it => it.id === id);
    if (item && !item.icon_data && url.startsWith('http')) {
      saveIconToDB(id, url);
    }
  }

  let iconAttempts = $state({});
  function handleIconError(item, e) {
    const currentAttempt = (iconAttempts[item.id] !== undefined) ? iconAttempts[item.id] : 0;
    if (currentAttempt < 2) {
      iconAttempts[item.id] = currentAttempt + 1;
      const nextUrl = getIconUrl(item, currentAttempt + 1);
      if (nextUrl) { e.currentTarget.src = nextUrl; return; }
    }
    e.currentTarget.style.display = 'none';
    loadedIcons[item.id] = false;
  }

  // --- INTERFAZ ---
  let expandedCardId = $state(null);
  let searchQuery = $state('');
  let visiblePasswords = $state(new Set());

  let filteredItems = $derived(
    items.filter(item => {
      const matchesSearch = item.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
                            item.username.toLowerCase().includes(searchQuery.toLowerCase());
      if (!matchesSearch) return false;
      if (activeFilter === 'favorites') return item.is_favorite;
      if (activeFilter === 'trash') return item.is_deleted;
      if (activeFilter.startsWith('folder-')) {
        const fId = parseInt(activeFilter.split('-')[1]);
        return item.folder_id === fId;
      }
      return !item.is_deleted;
    })
  );

  function togglePassword(id) {
    if (visiblePasswords.has(id)) visiblePasswords.delete(id);
    else visiblePasswords.add(id);
    visiblePasswords = new Set(visiblePasswords);
  }

  onMount(() => {
    loadAccounts();
    globalState.refreshTrashCount();
    startTOTPTimer();

    return () => {
      if (totpInterval) clearInterval(totpInterval);
    };
  });
</script>

<div class="w-full max-w-7xl mx-auto h-[85vh] transition-all duration-500 ease-in-out relative group/vault">
  
  {#if !showManager}
    <!-- VISTA 1: ACCESO RÁPIDO -->
    <div class="w-full h-full flex flex-col items-center justify-start pt-12 px-6">
      <div class="bg-[var(--md-surface)] h-20 rounded-full px-3 flex items-center border border-black/[0.1] dark:border-white/[0.15] mb-8 w-full max-w-2xl">
        <div class="w-14 h-14 rounded-full overflow-hidden shrink-0 border-2 p-0.5 ml-1" style="border-color: {user.color}66">
          <img src={user.avatar} alt="avatar" class="w-full h-full object-cover rounded-full bg-black/5 dark:bg-white/[0.1]" />
        </div>
        <div class="ml-4 flex flex-col flex-1 min-w-0 pr-4">
          <span class="text-sm font-bold truncate text-[var(--md-on-background)]">{user.name}</span>
          <div class="flex items-center gap-1.5 mt-0.5">
            <div class="w-1 h-1 rounded-full bg-green-500 animate-pulse"></div>
            <span class="text-[9px] font-black uppercase tracking-widest opacity-30 text-[var(--md-on-background)]">Sesión Activa</span>
          </div>
        </div>
        <div class="flex items-center gap-1 mr-2">
          <button onclick={onOpenNewEntry} class="p-3 rounded-full opacity-40 hover:opacity-100 transition-opacity hover:bg-black/5" style="color: {user.color}" title="Agregar Credencial" aria-label="Agregar credencial"><Plus size={18}/></button>
          <button onclick={onLock} class="p-3 rounded-full hover:bg-red-500/10 text-red-500 opacity-40 hover:opacity-100 transition-opacity" title="Bloquear" aria-label="Bloquear sesión"><LogOut size={18}/></button>
        </div>
      </div>

      <div class="w-full max-w-2xl relative mb-12">
        <Search class="absolute left-6 top-1/2 -translate-y-1/2 opacity-20 text-[var(--md-on-background)]" size={24} />
        <input 
          type="text" 
          bind:value={searchQuery} 
          placeholder="¿Qué necesitas buscar ahora?" 
          class="w-full h-20 bg-[var(--md-surface)] rounded-[32px] pl-16 pr-16 text-lg font-medium border border-black/[0.1] dark:border-white/[0.15] outline-none text-[var(--md-on-background)] transition-all" 
        />
        {#if searchQuery.length > 0}
          <button onclick={() => searchQuery = ''} class="absolute right-6 top-1/2 -translate-y-1/2 w-8 h-8 rounded-full flex items-center justify-center hover:bg-black/5 opacity-40 hover:opacity-100 transition-opacity" aria-label="Limpiar búsqueda"><X size={18} /></button>
        {/if}
      </div>

      <div class="w-full max-w-2xl flex-1 overflow-y-auto pb-32 space-y-4 custom-scrollbar px-4">
        {#if isLoading}
          <div class="space-y-4 px-4">
            {#each [1,2,3,4] as _}
              <div class="h-20 bg-[var(--md-surface-variant)] rounded-2xl animate-pulse opacity-50"></div>
            {/each}
          </div>
        {:else if activeFilter !== 'all' && searchQuery.length === 0}
          <div class="flex items-center justify-between mb-6 px-4">
            <div class="flex items-center gap-3">
              <div class="w-1.5 h-1.5 rounded-full bg-[var(--md-primary)]"></div>
              <span class="text-[10px] font-black uppercase tracking-[0.2em] opacity-40">Mostrando: {activeFilter}</span>
            </div>
            <button onclick={() => activeFilter = 'all'} class="flex items-center gap-2 text-[9px] font-black uppercase tracking-widest text-[#6750A4]">Ver todo</button>
          </div>
        {/if}

        <div class="flex flex-col gap-4 items-center w-full">
          {#each filteredItems as item (item.id)}
            <AccountCard 
              {item} 
              userColor={user.color} 
              expanded={expandedCardId === item.id}
              onToggleExpanded={() => expandedCardId = expandedCardId === item.id ? null : item.id}
              isFavorite={item.is_favorite}
              isDeleted={item.is_deleted}
              {totpProgress}
              {totpSeconds}
              onToggleFavorite={toggleFavorite}
              onTogglePassword={togglePassword}
              onRestore={restoreAccount}
              onEdit={editAccount}
              onDelete={deleteAccount}
              isVisible={visiblePasswords.has(item.id)}
              {iconMap}
              {loadedIcons}
              onIconLoad={handleIconLoad}
              onIconError={handleIconError}
              {getIconUrl}
            />
          {/each}
        </div>
      </div>

      <div class="absolute bottom-8 left-1/2 -translate-x-1/2 flex flex-col items-center gap-2">
        <span class="text-[8px] font-black uppercase tracking-[0.3em] opacity-20">Gestión Avanzada</span>
        <button onclick={() => showManager = true} class="w-14 h-14 bg-[var(--md-surface)] rounded-full border border-black/[0.1] dark:border-white/[0.1] flex items-center justify-center hover:scale-110 active:scale-95 transition-all group" style="color: {user.color}" aria-label="Abrir gestor de bóveda">
          <ChevronDown size={24} class="group-hover:translate-y-0.5 transition-transform" />
        </button>
      </div>
    </div>
  {:else}
    <!-- VISTA 2: GESTOR COMPLETO -->
    <div class="w-full h-full bg-[var(--md-surface)] rounded-[40px] border border-black/[0.1] dark:border-white/[0.15] flex flex-col overflow-hidden transition-all duration-500">
      
      <header class="w-full h-20 md:h-24 px-4 md:px-8 border-b border-black/[0.05] dark:border-white/[0.15] flex justify-between items-center shrink-0">
        <div class="flex items-center gap-2 md:gap-4">
          <button onclick={() => showManager = false} class="p-2 rounded-xl hover:bg-black/5 opacity-40 hover:opacity-100 mr-2" aria-label="Volver al acceso rápido"><ArrowLeft size={20} /></button>
          <div class="flex items-center gap-4">
            <div class="w-12 h-12 rounded-2xl overflow-hidden border-2 p-0.5 flex-shrink-0" style="border-color: {user.color}66">
              <img src={user.avatar} alt="avatar" class="w-full h-full object-cover rounded-xl bg-black/5 dark:bg-white/[0.1]" />
            </div>
            <div class="flex flex-col">
              <h2 class="text-base font-bold leading-none text-[var(--md-on-background)]">{user.name}</h2>
              <span class="text-[8px] font-black uppercase tracking-widest opacity-20 mt-1.5 text-[var(--md-on-background)]">Gestor de Bóveda</span>
            </div>
          </div>
        </div>
        <div class="flex items-center gap-1">
          <button onclick={openInbox} class="p-3 rounded-xl hover:bg-black/5 {totalNotifications > 0 ? 'text-[var(--md-primary)]' : 'opacity-20'} relative" aria-label="Abrir bandeja de pendientes">
            <Bell size={20} strokeWidth={1.5} />
            {#if totalNotifications > 0}<span class="absolute top-2 right-2 w-3.5 h-3.5 bg-red-500 text-white text-[7px] font-black rounded-full border-2 border-[var(--md-surface)] flex items-center justify-center animate-in zoom-in">{totalNotifications}</span>{/if}
          </button>
          <button onclick={onOpenSettings} class="p-3 rounded-xl hover:bg-black/5 opacity-20 hover:opacity-100 transition-opacity" aria-label="Abrir ajustes"><Settings size={20} strokeWidth={1.5} /></button>
        </div>
      </header>

      <div class="flex-1 flex flex-col-reverse md:flex-row overflow-hidden relative">
        <Sidebar 
          {folders} 
          bind:activeFilter 
          userColor={user.color} 
          {onOpenNewEntry} 
          {onOpenNewFolder} 
          onEmptyTrash={handleEmptyTrash}
          {iconMap}
        />

        <section class="flex-1 md:border-r border-black/[0.05] dark:border-white/[0.15] flex flex-col bg-[var(--md-surface)] overflow-hidden">
          <div class="p-4 md:p-6 border-b border-black/[0.05] dark:border-white/[0.15]">
            <div class="relative">
              <Search class="absolute left-5 top-1/2 -translate-y-1/2 opacity-20" size={16} />
              <input 
                type="text" 
                bind:value={searchQuery} 
                placeholder="Filtrar credenciales..." 
                class="h-14 bg-black/5 dark:bg-[var(--md-surface-variant)] rounded-2xl pl-12 pr-4 text-xs font-medium outline-none w-full border border-transparent focus:border-black/5" 
              />
            </div>
          </div>
          <div class="flex-1 overflow-y-auto p-3 space-y-1 custom-scrollbar">
            {#each filteredItems as item (item.id)}
              <div 
                role="button"
                tabindex="0"
                onclick={() => expandedCardId = item.id} 
                onkeydown={(e) => e.key === 'Enter' && (expandedCardId = item.id)}
                class="w-full p-4 flex items-center gap-4 text-left group hover:bg-black/[0.02] cursor-pointer transition-all rounded-[24px]"
                style={expandedCardId === item.id ? `background-color: ${user.color}0d; border: 1px solid ${user.color}1a;` : ''}
              >
                <div class="w-10 h-10 rounded-xl flex items-center justify-center font-bold text-xs text-white shrink-0 relative overflow-hidden" style="background-color: {item.icon_data || loadedIcons[item.id] ? 'transparent' : item.color}">
                  {#if item.icon_data}
                    <img src={item.icon_data} alt="" class="absolute inset-0 w-full h-full object-cover" />
                  {:else}
                    <img 
                      src={getIconUrl(item)} 
                      alt="" 
                      class="absolute inset-0 w-full h-full object-cover opacity-0 transition-opacity"
                      onload={(e) => handleIconLoad(item.id, getIconUrl(item), e)}
                      onerror={(e) => handleIconError(item, e)}
                    />
                  {/if}
                  <span class="relative z-10" class:opacity-0={item.icon_data || loadedIcons[item.id]}>{item.icon}</span>
                </div>
                <div class="flex flex-col flex-1 min-w-0">
                  <span class="text-xs font-bold truncate text-[var(--md-on-background)]">{item.title}</span>
                  <span class="text-[10px] opacity-30 truncate text-[var(--md-on-background)]">{item.username}</span>
                </div>
                {#if item.is_favorite}
                  <Star size={12} fill="currentColor" class="text-yellow-500 shrink-0" />
                {/if}
              </div>
            {/each}
          </div>
        </section>

        <!-- DETALLE DERECHA -->
        {#if expandedCardId}
          {@const sel = items.find(i => i.id === expandedCardId)}
          {#if sel}
            <aside class="hidden xl:flex w-96 flex-col bg-[var(--md-surface)] p-8 overflow-y-auto custom-scrollbar animate-in slide-in-from-right duration-500">
               <AccountCard 
                  item={sel} 
                  userColor={user.color} 
                  expanded={true}
                  onToggleExpanded={() => expandedCardId = null}
                  isFavorite={sel.is_favorite}
                  isDeleted={sel.is_deleted}
                  {totpProgress}
                  {totpSeconds}
                  onToggleFavorite={toggleFavorite}
                  onTogglePassword={togglePassword}
                  onRestore={restoreAccount}
                  onEdit={editAccount}
                  onDelete={deleteAccount}
                  isVisible={visiblePasswords.has(sel.id)}
                  {iconMap}
                  {loadedIcons}
                  onIconLoad={handleIconLoad}
                  onIconError={handleIconError}
                  {getIconUrl}
               />
            </aside>
            <!-- MOBILE OVERLAY PARA DETALLE -->
            <div class="xl:hidden absolute inset-0 z-50 bg-[var(--md-surface)] flex flex-col overflow-hidden animate-in fade-in zoom-in duration-300">
              <div class="p-4 border-b border-black/[0.05] dark:border-white/[0.15] shrink-0">
                <button onclick={() => expandedCardId = null} class="p-2 rounded-xl hover:bg-black/5 opacity-60 hover:opacity-100 flex items-center gap-2 transition-all">
                  <ArrowLeft size={20} /> <span class="font-bold text-xs uppercase tracking-widest">Volver</span>
                </button>
              </div>
              <div class="flex-1 overflow-y-auto p-4 sm:p-8 custom-scrollbar">
                <AccountCard 
                  item={sel} 
                  userColor={user.color} 
                  expanded={true}
                  onToggleExpanded={() => expandedCardId = null}
                  isFavorite={sel.is_favorite}
                  isDeleted={sel.is_deleted}
                  {totpProgress}
                  {totpSeconds}
                  onToggleFavorite={toggleFavorite}
                  onTogglePassword={togglePassword}
                  onRestore={restoreAccount}
                  onEdit={editAccount}
                  onDelete={deleteAccount}
                  isVisible={visiblePasswords.has(sel.id)}
                  {iconMap}
                  {loadedIcons}
                  onIconLoad={handleIconLoad}
                  onIconError={handleIconError}
                  {getIconUrl}
               />
              </div>
            </div>
          {/if}
        {/if}
      </div>
    </div>
  {/if}

  <!-- BANDEJA DE PENDIENTES (SECURE INBOX) -->
  {#if isInboxOpen}
    <div class="fixed inset-0 z-[150] flex items-center justify-center p-6">
      <button class="absolute inset-0 bg-black/50 backdrop-blur-sm border-none w-full h-full cursor-default" onclick={() => isInboxOpen = false} aria-label="Cerrar bandeja"></button>
      <div class="w-full max-w-[440px] max-h-[80vh] bg-[var(--md-surface)] rounded-[32px] border border-black/10 dark:border-white/10 relative z-10 flex flex-col overflow-hidden shadow-2xl">
        <div class="px-6 py-5 flex items-center justify-between border-b border-black/5 dark:border-white/10">
          <h3 class="text-sm font-black uppercase tracking-[0.2em] opacity-40">Bandeja</h3>
          <button onclick={() => isInboxOpen = false} class="p-2 rounded-xl hover:bg-black/5" aria-label="Cerrar"><X size={18} /></button>
        </div>
        <div class="flex-1 overflow-y-auto p-4 space-y-2 custom-scrollbar">
          {#if isInboxLoading}
            <div class="p-8 text-center text-xs opacity-40">Cargando...</div>
          {:else if inboxItems.length === 0}
            <div class="p-8 text-center opacity-40">
              <Bell size={24} class="mx-auto mb-3 opacity-40" />
              <p class="text-xs font-bold">No hay elementos pendientes.</p>
            </div>
          {:else}
            {#each inboxItems as it (it.id)}
              <div class="flex items-center justify-between gap-3 p-3 rounded-2xl bg-[var(--md-surface-variant)]">
                <div class="flex flex-col min-w-0">
                  <span class="text-xs font-bold truncate">{it.label}</span>
                  <span class="text-[10px] opacity-30 truncate">{it.username}</span>
                </div>
                <button onclick={() => deleteInboxItem(it.id)} class="p-2 rounded-xl hover:bg-red-500/10 text-red-500 shrink-0" aria-label="Descartar elemento"><Trash2 size={16} /></button>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>
