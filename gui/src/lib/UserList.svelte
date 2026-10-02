<script>
  import { Plus } from 'lucide-svelte';
  import { globalState } from './state.svelte.js';
  let { onSelect, selectedId, isMobile, users = $bindable(), onAddClick, onQuickEntry } = $props();

  function selectUser(user) {
    onSelect(user);
  }

  /** @param {any} e */
  function handleAvatarError(e) {
    const img = e.currentTarget;
    img.style.display = 'none';
    const next = img.nextElementSibling;
    if (next) {
      next.style.display = 'flex';
    }
  }

  // Función para obtener variables de color individuales siguiendo M3
  function getUserStyles(hex) {
    if (!hex) return '';
    
    // HEX a HSL (Matemática simplificada para la lista)
    let r = parseInt(hex.slice(1, 3), 16) / 255;
    let g = parseInt(hex.slice(3, 5), 16) / 255;
    let b = parseInt(hex.slice(5, 7), 16) / 255;
    let max = Math.max(r, g, b), min = Math.min(r, g, b);
    let h = 0, s = 0, l = (max + min) / 2;
    if (max === min) {
      h = s = 0;
    } else {
      let d = max - min;
      s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
      switch (max) {
        case r: h = (g - b) / d + (g < b ? 6 : 0); break;
        case g: h = (b - r) / d + 2; break;
        case b: h = (r - g) / d + 4; break;
      }
      h /= 6;
    }
    h *= 360; s *= 100;

    const isDark = globalState.isDark;
    const targetS = isDark ? Math.min(s, 40) : s;
    
    const primary = `hsl(${h}, ${targetS}%, ${isDark ? 80 : 40}%)`;
    const onPrimary = `hsl(${h}, ${targetS}%, ${isDark ? 20 : 100}%)`;
    const container = `hsl(${h}, ${targetS}%, ${isDark ? 30 : 90}%)`;
    const onContainer = `hsl(${h}, ${targetS}%, ${isDark ? 90 : 10}%)`;

    return `--u-primary: ${primary}; --u-on-primary: ${onPrimary}; --u-container: ${container}; --u-on-container: ${onContainer};`;
  }
</script>

<div class="flex flex-col gap-1 w-full">
  {#each users as user}
    <div class="group/item relative" style={getUserStyles(user.color)}>
      <button 
        onclick={() => selectUser(user)} 
        class="flex items-center gap-3 p-2 pr-12 transition-[background-color,border-color,opacity] duration-500 ease-in-out text-left relative w-full rounded-full
               {selectedId === user.id
                 ? 'bg-[var(--u-container)] border border-[var(--u-primary)]/10 text-[var(--u-on-container)]' 
                 : 'hover:bg-[var(--u-container)]/30 opacity-60 hover:opacity-100'}"
      >
        <div class="relative shrink-0">
          <div 
            class="w-10 h-10 rounded-full overflow-hidden border-2 transition-[border-color,transform] duration-500 ease-in-out {selectedId === user.id ? 'scale-105' : 'border-transparent'}"
            style={selectedId === user.id ? 'border-color: var(--u-primary)' : ''}
          >
            <img 
              src={user.avatar} 
              alt="avatar" 
              class="w-full h-full object-cover bg-[var(--u-container)]" 
              onerror={handleAvatarError}
            />
            <div 
              class="w-full h-full items-center justify-center font-bold text-sm hidden"
              style="background-color: var(--u-primary); color: var(--u-on-primary);"
            >
              {user.name.substring(0, 1).toUpperCase()}
            </div>
          </div>
          <div 
            class="absolute -bottom-1 -right-1 w-4 h-4 rounded-md border border-[var(--md-surface)] flex items-center justify-center text-[7px] font-black shadow-sm"
            style="background-color: var(--u-primary); color: var(--u-on-primary);"
          >
            {user.name.substring(0, 1).toUpperCase()}
          </div>
        </div>

        <div class="flex flex-col min-w-0 flex-grow">
          <span class="text-xs font-display font-black tracking-tight truncate" style="color: inherit">{user.name}</span>
        </div>
      </button>

      <!-- Botón Quick Entry -->
      <button 
        onclick={(e) => { e.stopPropagation(); onQuickEntry?.(user); }}
        class="absolute right-2 top-1/2 -translate-y-1/2 p-2 rounded-full opacity-0 group-hover/item:opacity-100 transition-[background-color,color,opacity] duration-500 ease-in-out"
        style="color: var(--u-primary); --hover-bg: var(--u-primary)1a;"
        onmouseenter={(e) => e.currentTarget.style.backgroundColor = 'var(--hover-bg)'}
        onmouseleave={(e) => e.currentTarget.style.backgroundColor = 'transparent'}
        title="Agregar Credencial Rápida"
        aria-label="Agregar credencial rápida"
      >
        <Plus size={16} />
      </button>
    </div>
  {/each}

  <!-- Add User Button -->
  <button 
    onclick={(e) => { e.stopPropagation(); onAddClick?.(); }}
    class="flex items-center gap-3 p-2 pr-4 mt-2 transition-[background-color,border-color,opacity] duration-500 ease-in-out border border-dashed border-black/[0.05] dark:border-white/[0.05] opacity-30 hover:opacity-100 hover:border-[var(--md-primary)]/20 hover:bg-white/30 dark:hover:bg-white/[0.01] w-full rounded-full"
    title="Agregar Usuario"
  >
    <div class="w-10 h-10 rounded-full flex items-center justify-center flex-shrink-0 text-[var(--md-on-background)] transition-colors duration-500 ease-in-out">
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M12 4v16m8-8H4" />
      </svg>
    </div>
    <span class="text-[10px] font-ui font-black uppercase tracking-widest text-[var(--md-on-background)]">Nuevo Usuario</span>
  </button>
</div>
