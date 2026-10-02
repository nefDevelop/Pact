<script>
  import { Lock } from 'lucide-svelte';
  import { globalState } from './state.svelte.js';
  import { api } from './api.js';
  import { deriveKeys } from './crypto_utils.js';
  import { i18n } from './i18n.svelte.js';
  let { user, onUnlock } = $props();
  let password = $state('');
  let isLoading = $state(false);

  $effect(() => {
    const username = user.username || user.name;
    if (user && username && (!globalState.selectedUser || globalState.selectedUser.username !== username)) {
      globalState.selectedUser = { ...user, username };
      globalState.userColor = user.color;
    }
  });

  async function handleLogin() {
    if (!password) return;
    isLoading = true;
    
    try {
      const username = user.username || user.name;
      
      const { salt } = await api.get(`/auth/salt/${username}`);

      const { masterKey, authKey, syncKey, syncKeyHex } = await deriveKeys(password, salt);

      const data = await api.post('/auth/unlock', {
        username: username,
        master_password: authKey 
      });

      globalState.masterKey = masterKey;
      globalState.syncKey = syncKey;
      globalState.syncKeyHex = syncKeyHex;
      globalState.authToken = data.token;
      globalState.userColor = user.color;

      // Ajusta el modo (local/remoto/híbrido) a partir de la config guardada.
      // El auto-sync lo arranca App.svelte al detectar la bóveda desbloqueada.
      await globalState.refreshSyncConfig();

      onUnlock?.();
    } catch (error) {
      const msg = error.message === 'Unauthorized'
        ? i18n.t('login.incorrect')
        : (error.message || i18n.t('login.failed'));
      globalState.notify(i18n.t('login.denied'), msg, 'error');
      password = '';
    } finally {
      isLoading = false;
    }
  }

  function handleAvatarError(e) {
    const img = e.currentTarget;
    img.style.display = 'none';
    const next = img.nextElementSibling;
    if (next) {
      next.style.display = 'flex';
    }
  }
</script>

<div class="rounded-[48px] p-12 border border-border shadow-elev-3 transition-all duration-500 ease-in-out w-full max-w-[440px] relative z-10"
     style="background-color: var(--md-surface-2); color: var(--md-on-surface);">
  
  <div class="flex items-center gap-6 mb-10">
    <div class="w-20 h-20 rounded-full p-1 flex items-center justify-center shrink-0 relative">
      
      <svg class="absolute inset-0 w-full h-full animate-spin-slow" viewBox="0 0 100 100" style="--duration: 60s;">
        <circle 
          cx="50" cy="50" r="48" 
          fill="none" 
          stroke={user.color} 
          stroke-width="2" 
          pathLength="100"
          stroke-dasharray="4 2.25" 
          class="opacity-40"
        />
      </svg>

      <img 
        src={user.avatar} 
        alt="avatar" 
        class="w-[calc(100%-8px)] h-[calc(100%-8px)] rounded-full object-cover bg-[var(--md-primary-container)] relative z-10" 
        onerror={handleAvatarError}
      />
      <div 
        class="w-[calc(100%-8px)] h-[calc(100%-8px)] rounded-full items-center justify-center font-bold hidden relative z-10"
        style="background-color: var(--md-primary); color: var(--md-on-primary);"
      >
        {user.name.substring(0, 1).toUpperCase()}
      </div>
    </div>

    <div class="flex flex-col">
      <h2 class="text-3xl font-display font-black tracking-tight leading-none" style="color: var(--md-on-surface)">{user.name}</h2>
      <div class="flex items-center gap-3 mt-2">
        <div class="flex items-center gap-2">
          <Lock size={12} class="opacity-40" style="color: {user.color}" />
          <span class="text-[10px] font-ui font-medium uppercase tracking-widest opacity-40" style="color: var(--md-on-surface)">{i18n.t('login.locked')}</span>
        </div>
      </div>
    </div>
  </div>

  <form onsubmit={(e) => { e.preventDefault(); handleLogin(); }} class="space-y-8">
    <div class="space-y-4">
      <label for="pass" class="text-[10px] font-ui font-medium uppercase tracking-widest opacity-60 px-2" style="color: var(--md-on-surface)">{i18n.t('login.masterPassword')}</label>
      <input 
        id="pass"
        type="password" 
        bind:value={password}
        required
        class="w-full h-16 bg-[var(--md-surface-variant)] rounded-2xl px-6 focus:outline-none focus:ring-2 focus:ring-[var(--md-primary)]/20 border border-transparent focus:border-[var(--md-primary)]/10 text-sm font-body font-bold"
        style="color: var(--md-on-surface)"
        placeholder={i18n.t('login.placeholder')}
      />
    </div>

    <button 
      type="submit"
      disabled={isLoading}
      class="w-full h-16 rounded-full font-ui font-medium text-sm uppercase tracking-[0.2em] transition-all duration-500 ease-in-out mt-4 active:scale-[0.98] disabled:opacity-50 overflow-hidden relative group shadow-elev-2"
      style="background-color: var(--md-primary); color: var(--md-on-primary);"
    >
      <div class="absolute inset-0 bg-white/10 opacity-0 group-hover:opacity-100 transition-opacity"></div>
      <div class="relative flex items-center justify-center gap-3">
        {#if isLoading}
          <div class="w-5 h-5 rounded-full border-2 border-white/20 border-t-white animate-spin"></div>
          <div class="flex flex-col items-start leading-none">
            <span class="text-[10px] font-black tracking-widest">{i18n.t('login.verifying')}</span>
            <span class="text-[8px] opacity-60 font-bold">{i18n.t('login.argon')}</span>
          </div>
        {:else}
          <span>{i18n.t('login.unlock')}</span>
        {/if}
      </div>
    </button>
  </form>
</div>

<style>
  @keyframes spin-slow {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  .animate-spin-slow {
    animation: spin-slow var(--duration, 60s) linear infinite;
  }
</style>
