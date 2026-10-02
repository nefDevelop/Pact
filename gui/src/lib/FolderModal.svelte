<script>
  import { Folder, Shield, Lock, Ghost, Gamepad2, Tv, Music, Coffee, Heart, Star, ShoppingBag, Luggage, Wallet, Umbrella, X } from 'lucide-svelte';
  import { api } from './api.js';
  import { globalState } from './state.svelte.js';
  import { encryptE2E } from './crypto_utils.js';
  import { focusTrap } from './focusTrap.js';
  import { i18n } from './i18n.svelte.js';
  
  let { isOpen = $bindable(), userColor, onFolderCreated } = $props();
  
  let newFolderName = $state('');
  let selectedIcon = $state('folder');

  const availableIcons = [
    { name: 'folder', component: Folder },
    { name: 'shield', component: Shield },
    { name: 'lock', component: Lock },
    { name: 'ghost', component: Ghost },
    { name: 'gamepad2', component: Gamepad2 },
    { name: 'tv', component: Tv },
    { name: 'music', component: Music },
    { name: 'coffee', component: Coffee },
    { name: 'heart', component: Heart },
    { name: 'star', component: Star },
    { name: 'shopping-bag', component: ShoppingBag },
    { name: 'luggage', component: Luggage },
    { name: 'wallet', component: Wallet },
    { name: 'umbrella', component: Umbrella }
  ];

  async function confirmCreateFolder() {
    if (!newFolderName) return;
    if (!globalState.masterKey) {
      globalState.notify('Error de Seguridad', 'La bóveda está bloqueada.', 'error');
      return;
    }
    try {
      // El nombre de la carpeta se cifra E2E antes de enviarlo.
      const encryptedName = await encryptE2E(newFolderName.trim(), globalState.masterKey);
      await api.post('/vault/folders', {
        name: encryptedName,
        color: userColor || '#6750A4',
        icon: selectedIcon
      });
      newFolderName = '';
      isOpen = false;
      onFolderCreated?.();
    } catch (e) { /* Error handled by api.js */ }
  }
</script>

{#if isOpen}
  <div class="fixed inset-0 z-[200] flex items-center justify-center p-6">
    <div 
      class="absolute inset-0 bg-black/60 backdrop-blur-md animate-in fade-in" 
      onclick={() => isOpen = false}
      onkeydown={(e) => e.key === 'Escape' && (isOpen = false)}
      role="button"
      tabindex="-1"
      aria-label="Cerrar modal"
    ></div>
    <div use:focusTrap role="dialog" aria-modal="true" aria-label="Nueva carpeta" class="w-full max-w-[400px] bg-[var(--md-surface-2)] rounded-[32px] border border-border-window relative z-10 p-8 shadow-elev-3 animate-in zoom-in duration-300">
      <div class="flex justify-between items-start mb-2">
        <div>
          <h3 class="text-xl font-bold">{i18n.t('folder.title')}</h3>
          <p class="text-[10px] font-black uppercase tracking-widest text-[var(--md-on-surface-variant)]">{i18n.t('folder.subtitle')}</p>
        </div>
        <button onclick={() => isOpen = false} class="p-2 opacity-20 hover:opacity-100 transition-opacity">
          <X size={20} />
        </button>
      </div>

      <div class="space-y-6 mt-6">
        <input 
          type="text" 
          bind:value={newFolderName} 
          placeholder={i18n.t('folder.placeholder')} 
          class="w-full h-12 bg-[var(--md-surface-variant)] rounded-2xl px-5 focus:outline-none font-bold border border-transparent focus:border-[var(--md-primary)]/30 transition-colors" 
        />
        
        <div class="grid grid-cols-7 gap-2 px-1">
          {#each availableIcons as icon}
            <button 
              onclick={() => selectedIcon = icon.name} 
              class="w-10 h-10 rounded-xl flex items-center justify-center transition-all {selectedIcon === icon.name ? 'bg-[var(--md-primary)] text-[var(--md-on-primary)] shadow-elev-2 scale-110' : 'bg-[var(--md-surface-variant)] opacity-40 hover:opacity-100'}"
              title={icon.name}
            >
              <icon.component size={18} />
            </button>
          {/each}
        </div>

        <div class="flex justify-end gap-3 pt-2">
          <button onclick={() => isOpen = false} class="px-6 h-12 rounded-full text-[10px] font-black uppercase tracking-widest opacity-40 hover:opacity-100 transition-opacity">{i18n.t('common.cancel')}</button>
          <button 
            onclick={confirmCreateFolder} 
            class="px-8 h-12 rounded-full bg-[var(--md-primary)] text-[var(--md-on-primary)] text-[10px] font-black uppercase tracking-widest shadow-elev-2 active:scale-95 transition-all disabled:opacity-30"
            disabled={!newFolderName}
          >
            {i18n.t('common.create')}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
