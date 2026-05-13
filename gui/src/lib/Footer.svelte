<script>
  import { Loader2, Sun, Moon, ArrowRightLeft, HardDrive, Cloud } from 'lucide-svelte';
  import { globalState } from './state.svelte.js';
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { cubicInOut } from 'svelte/easing';

  // Determinar qué icono mostrar según el modo de base de datos
  let modeIcon = $derived(() => {
    if (globalState.dbMode === 'remote') return Cloud;
    if (globalState.dbMode === 'hybrid') return ArrowRightLeft;
    return HardDrive; // Default local
  });
  
  const CurrentIcon = $derived(modeIcon());

  function toggleTheme() {
    globalState.isDark = !globalState.isDark;
  }

  $effect(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.classList.toggle('dark', globalState.isDark);
    }
  });

  onMount(() => {
    if (typeof document !== 'undefined') {
      globalState.isDark = document.documentElement.classList.contains('dark') || 
                           window.matchMedia('(prefers-color-scheme: dark)').matches;
    }
  });

  async function sync_all() {
    await globalState.performSync(true);
  }
</script>

<footer class="w-full flex justify-between items-center text-[10px] font-black uppercase tracking-widest text-[var(--md-on-surface-variant)] px-10 lg:px-16 transition-all duration-500 ease-in-out bg-transparent border-none">
  
  <div class="flex items-center gap-8">
    <!-- LOGO DE PACT (Altura igualada al bloque de texto) -->
    <div class="flex items-center gap-3 py-1.5">
      <div class="w-10 h-10 rounded-2xl flex items-center justify-center shadow-lg shadow-[#6750A4]/10 overflow-hidden p-2 transition-colors duration-500" 
           style="background-color: var(--md-primary); color: {globalState.isDark ? '#000000' : '#ffffff'};">
        <svg
          width="100%"
          height="100%"
          viewBox="0 0 52.111928 52.145615"
          version="1.1"
          xml:space="preserve"
          xmlns="http://www.w3.org/2000/svg"
          style="fill: currentColor;">
          <g transform="translate(-42.365272,-123.74772)">
            <g>
              <path
                style="fill:none;stroke:currentColor;stroke-width:4.52528;stroke-linecap:square;stroke-opacity:1"
                d="m 219.44579,92.531097 h 34.86993 a 6.5125923,6.5125923 23.293288 0 1 4.73084,2.036763 l 2.87737,3.041316 a 5.528695,5.528695 68.293288 0 1 1.51258,3.799634 v 14.50574 a 6.4148793,6.4148793 116.33279 0 1 -2.52447,5.10053 l -3.55002,2.70776 a 6.4148793,6.4148793 116.33279 0 0 -2.52447,5.10053 v 6.9728 c 0,1.7535 0.14402,4.73269 -0.7371,6.22483 -0.24472,0.41441 -0.54684,0.77123 -0.90102,1.06526 -1.32179,1.09731 -4.37554,0.97424 -6.12904,0.97424 h -26.89895 c -1.7535,0 -4.72338,0.22558 -6.26972,-0.56696 -0.6137,-0.31454 -1.0868,-0.77708 -1.4179,-1.42891 -0.78705,-1.54947 -0.57672,-4.51496 -0.57672,-6.26846 v -35.72639 c 0,-1.352738 -0.0982,-3.577557 0.26148,-4.877465 0.34899,-1.261102 1.08184,-2.024723 2.40063,-2.392862 1.29903,-0.362623 3.52384,-0.268356 4.87658,-0.268356 z"
                transform="matrix(0.92895309,0,0,0.92907739,-152.34488,39.919423)" />
              <g
                style="fill:currentColor;fill-opacity:1;stroke:none;stroke-width:8.55171"
                transform="matrix(0.92895309,0,0,0.92907739,-68.216913,88.285856)">
                <circle
                   style="opacity:1;fill:currentColor;fill-opacity:1;stroke:none;stroke-width:4.52528;stroke-linecap:round;stroke-linejoin:round;stroke-miterlimit:28.2;stroke-dasharray:none;stroke-dashoffset:14.8;stroke-opacity:1"
                   cx="146.67404"
                   cy="60.504368"
                   r="11.443624" />
                <rect
                   style="opacity:1;fill:currentColor;fill-opacity:1;stroke:none;stroke-width:4.52528;stroke-linecap:round;stroke-linejoin:round;stroke-miterlimit:28.2;stroke-dasharray:none;stroke-dashoffset:14.8;stroke-opacity:1"
                   width="10.054167"
                   height="20.475454"
                   x="141.65431"
                   y="62.938229"
                   rx="5.0893312" />
              </g>
            </g>
          </g>
        </svg>
      </div>
      <div class="flex flex-col items-start leading-none justify-center">
        <span class="text-lg font-display font-black tracking-tighter text-[var(--md-on-background)]">PACT</span>
        <div class="flex items-center gap-1 mt-1">
          <span class="text-[7px] opacity-60 font-ui font-medium uppercase tracking-[0.2em]">Secure Vault</span>
        </div>
      </div>
    </div>
  </div>

  <div class="flex items-center gap-1">
    <button 
      onclick={sync_all}
      disabled={globalState.isSyncing}
      class="flex items-center gap-2 px-3 py-2 rounded-full hover:bg-black/5 dark:hover:bg-white/5 transition-all active:scale-95 disabled:opacity-50 relative group"
      title="Estado de sincronización"
    >
      <div class="relative flex items-center justify-center w-8 h-8">
        <!-- Anillo de Tiempo Restante (SVG) -->
        <svg class="absolute inset-0 -rotate-90 w-8 h-8 pointer-events-none">
          <circle
            cx="16" cy="16" r="14"
            fill="none"
            stroke="currentColor"
            stroke-width="1"
            class="opacity-[0.05]"
          />
          <circle
            cx="16" cy="16" r="14"
            fill="none"
            stroke={globalState.remainingSeconds < 60 ? 'var(--md-error)' : 'var(--md-primary)'}
            stroke-width="1"
            stroke-dasharray="88"
            stroke-dashoffset={88 - (88 * globalState.remainingSeconds) / (globalState.autoLockMinutes * 60)}
            class="transition-all duration-1000 ease-linear opacity-40"
          />
        </svg>

        {#if globalState.isSyncing}
          <div in:fade={{ duration: 200 }} out:fade={{ duration: 200 }} class="flex items-center justify-center">
             <Loader2 size={16} class="animate-spin" style="color: var(--md-primary)" />
          </div>
        {:else}
          <CurrentIcon size={16} class={globalState.hasConflicts ? 'text-amber-500' : 'opacity-30'} />
          {#if globalState.hasConflicts}
            <span class="absolute -top-1 -right-1 w-1.5 h-1.5 bg-red-500 rounded-full border border-white dark:border-[#1c1b1f]"></span>
          {/if}
        {/if}
      </div>

      <div class="flex flex-col items-start leading-none min-w-[70px]">
        {#if globalState.isSyncing}
          <span in:fly={{ y: 5, duration: 300 }} class="text-[8px] font-black uppercase tracking-widest text-[var(--md-primary)]">Sincronizando...</span>
        {:else}
          <span in:fade class="text-[9px] font-black uppercase tracking-widest opacity-40">
            {globalState.dbMode === 'hybrid' ? 'Híbrido' : (globalState.dbMode === 'remote' ? 'Online' : 'Local')}
          </span>
        {/if}
      </div>
    </button>

    <!-- Interruptor de Tema -->
    <button 
      onclick={toggleTheme}
      class="flex items-center gap-2 px-3 py-2 rounded-full hover:bg-black/5 dark:hover:bg-white/5 transition-all duration-500 ease-in-out active:scale-95"
      title="Cambiar tema"
    >
      <div class="relative w-4 h-4 shrink-0">
        {#if globalState.isDark}
          <div in:fade={{ duration: 400 }} out:fade={{ duration: 300 }} class="absolute inset-0">
            <Moon size={16} style="color: var(--md-primary)" />
          </div>
        {:else}
          <div in:fade={{ duration: 400 }} out:fade={{ duration: 300 }} class="absolute inset-0">
            <Sun size={16} class="text-amber-400" />
          </div>
        {/if}
      </div>
      
      <div class="overflow-hidden flex flex-col h-3 relative min-w-[45px] text-center">
        {#if globalState.isDark}
          <span in:fly={{ y: 10, duration: 400 }} out:fly={{ y: -10, duration: 400 }} class="absolute inset-0 text-[9px] font-ui font-medium uppercase tracking-widest opacity-50">Oscuro</span>
        {:else}
          <span in:fly={{ y: 10, duration: 400 }} out:fly={{ y: -10, duration: 400 }} class="absolute inset-0 text-[9px] font-ui font-medium uppercase tracking-widest opacity-50">Claro</span>
        {/if}
      </div>
    </button>
  </div>
</footer>

<style>
  /* Footer alineado y profesional */
</style>
