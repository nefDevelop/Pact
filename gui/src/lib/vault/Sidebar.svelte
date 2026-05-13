<script>
  import { Grid, Star, Plus, Folder, Trash2 } from 'lucide-svelte';
  
  let { 
    folders, 
    activeFilter = $bindable(), 
    userColor, 
    onOpenNewEntry, 
    onOpenNewFolder, 
    onEmptyTrash,
    iconMap
  } = $props();

  function applyStyle(e, property, value) {
    e.currentTarget.style[property] = value;
  }
</script>

<aside class="w-20 lg:w-64 bg-[var(--md-surface-variant)] border-r border-black/[0.05] dark:border-white/[0.15] flex flex-col py-6 transition-[background-color,border-color] duration-500 ease-in-out">
  <div class="px-3 lg:px-4 mb-6">
    <button 
      onclick={onOpenNewEntry} 
      class="w-full h-12 rounded-2xl text-white flex items-center justify-center lg:justify-start lg:px-4 gap-0 lg:gap-3 hover:scale-[1.02] transition-all" 
      style="background-color: var(--md-primary); color: var(--md-on-primary);" 
      title="Nuevo Ítem"
    >
      <Plus size={18} />
      <span class="hidden lg:block font-bold text-[10px] uppercase tracking-widest">Nuevo Ítem</span>
    </button>
  </div>
  
  <nav class="px-2 lg:px-4 space-y-1 flex-1 overflow-y-auto custom-scrollbar">
    <button 
      onclick={() => activeFilter = 'all'}
      class="w-full h-11 rounded-xl flex items-center justify-center lg:justify-start lg:px-4 gap-0 lg:gap-4 transition-all {activeFilter === 'all' ? '' : 'opacity-40 hover:opacity-100 hover:bg-black/5'}"
      style={activeFilter === 'all' ? 'background-color: var(--md-primary-container); color: var(--md-on-primary-container);' : ''}
    >
      <Grid size={16} />
      <span class="hidden lg:block font-ui font-bold text-[9px] uppercase tracking-widest">Todos</span>
    </button>
    
    <button 
      onclick={() => activeFilter = 'favorites'}
      class="w-full h-11 rounded-xl flex items-center justify-center lg:justify-start lg:px-4 gap-0 lg:gap-4 transition-all {activeFilter === 'favorites' ? '' : 'opacity-40 hover:opacity-100 hover:bg-black/5'}"
      style={activeFilter === 'favorites' ? 'background-color: var(--md-primary-container); color: var(--md-on-primary-container);' : ''}
    >
      <Star size={16} fill={activeFilter === 'favorites' ? "currentColor" : "none"} />
      <span class="hidden lg:block font-ui font-bold text-[9px] uppercase tracking-widest">Favoritos</span>
    </button>

    <div class="h-px bg-black/[0.05] dark:bg-white/[0.05] my-4 mx-2"></div>
    
    <div class="hidden lg:flex items-center justify-between px-4 mb-2">
      <p class="text-[8px] font-black uppercase tracking-[0.2em] opacity-20">Carpetas</p>
      <button 
        onclick={onOpenNewFolder} 
        class="p-1 rounded-md hover:bg-black/5 dark:hover:bg-white/5 opacity-20 hover:opacity-100 transition-colors" 
        style="--hover-color: {userColor};" 
        onmouseenter={(e) => applyStyle(e, 'color', 'var(--hover-color)')} 
        onmouseleave={(e) => applyStyle(e, 'color', '')} 
        title="Nueva Carpeta"
      >
        <Plus size={12} />
      </button>
    </div>
    
    {#each folders.filter(f => f.uid !== 'system-trash') as f}
      {@const SidebarIcon = iconMap[f.icon] || Folder}
      <button 
        onclick={() => activeFilter = `folder-${f.id}`}
        class="w-full h-11 rounded-xl flex items-center justify-center lg:justify-start lg:px-4 gap-0 lg:gap-4 transition-all {activeFilter === `folder-${f.id}` ? '' : 'opacity-40 hover:opacity-100 hover:bg-black/5'}"
        style={activeFilter === `folder-${f.id}` ? 'background-color: var(--md-primary-container); color: var(--md-on-primary-container);' : ''}
      >
        <SidebarIcon size={16} style="color: {f.color}" />
        <span class="hidden lg:block font-ui font-bold text-[9px] uppercase tracking-widest">{f.name}</span>
      </button>
    {/each}
  </nav>

  <div class="px-2 lg:px-4 mt-auto pt-4">
    {#if activeFilter === 'trash'}
      <button 
        onclick={onEmptyTrash}
        class="w-full h-11 rounded-xl flex items-center justify-center lg:justify-start lg:px-4 gap-0 lg:gap-4 transition-all bg-red-600 text-white shadow-lg shadow-red-500/20 active:scale-95 group"
      >
        <Trash2 size={16} class="group-hover:scale-110 transition-transform" />
        <span class="hidden lg:block font-ui font-bold text-[9px] uppercase tracking-widest">Vaciar Papelera</span>
      </button>
    {:else}
      <button 
        onclick={() => activeFilter = 'trash'}
        class="w-full h-11 rounded-xl flex items-center justify-center lg:justify-start lg:px-4 gap-0 lg:gap-4 transition-all opacity-40 hover:opacity-100 hover:bg-black/5"
      >
        <Trash2 size={16} />
        <span class="hidden lg:block font-ui font-bold text-[9px] uppercase tracking-widest">Papelera</span>
      </button>
    {/if}
  </div>
</aside>
