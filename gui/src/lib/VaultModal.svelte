<script>
  import { X } from 'lucide-svelte';
  import { api } from './api.js';
  import { globalState } from './state.svelte.js';
  
  let { isOpen = $bindable(), onVaultCreated } = $props();
  let newVaultName = $state('familia');

  async function confirmNamingVault() {
    if (!newVaultName) return;
    try {
      const data = await api.post('/vault/initialize', { name: newVaultName });
      globalState.vaultPath = data.path;
      isOpen = false;
      onVaultCreated?.();
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
    <div class="w-full max-w-[400px] bg-white dark:bg-[#16151a] rounded-[32px] border border-black/[0.1] dark:border-white/[0.1] relative z-10 p-8 shadow-2xl animate-in zoom-in duration-300">
      <div class="flex justify-between items-start mb-2">
        <div>
          <h3 class="text-xl font-bold">Nueva Bóveda</h3>
          <p class="text-[10px] font-black uppercase tracking-widest opacity-30">Elige un nombre para tu archivo</p>
        </div>
        <button onclick={() => isOpen = false} class="p-2 opacity-20 hover:opacity-100 transition-opacity">
          <X size={20} />
        </button>
      </div>

      <div class="space-y-4 mt-6">
        <input 
          type="text" 
          bind:value={newVaultName} 
          placeholder="Nombre de la bóveda" 
          class="w-full h-12 bg-black/5 dark:bg-white/5 rounded-2xl px-5 focus:outline-none font-bold border border-transparent focus:border-[var(--md-primary)] transition-colors" 
        />
        <div class="flex justify-end gap-3 pt-4">
          <button onclick={() => isOpen = false} class="px-6 h-12 rounded-full text-[10px] font-black uppercase tracking-widest opacity-40 hover:opacity-100 transition-opacity">Cancelar</button>
          <button 
            onclick={confirmNamingVault} 
            class="px-8 h-12 rounded-full bg-[#6750A4] text-white text-[10px] font-black uppercase tracking-widest shadow-lg active:scale-95 transition-all disabled:opacity-30"
            disabled={!newVaultName}
          >
            Crear
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
