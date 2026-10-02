<script>
  import { Grid, Star, Plus, Folder, Trash2 } from 'lucide-svelte';
  import { i18n } from '../i18n.svelte.js';
  
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

<aside class="w-full md:w-20 lg:w-64 bg-[var(--md-surface-variant)] border-t md:border-t-0 border-r-0 md:border-r border-border flex flex-row md:flex-col p-2 md:py-6 transition-[background-color,border-color] duration-500 ease-in-out shrink-0 z-20 overflow-x-auto md:overflow-x-visible custom-scrollbar gap-2 md:gap-0 items-center md:items-stretch">
  <div class="px-1 md:px-3 lg:px-4 md:mb-6 shrink-0">
    <button 
      onclick={onOpenNewEntry} 
      class="w-12 md:w-full h-12 rounded-2xl text-white flex items-center justify-center lg:justify-start lg:px-4 gap-0 lg:gap-3 hover:scale-[1.02] transition-all shrink-0" 
      style="background-color: var(--md-primary); color: var(--md-on-primary);" 
      title={i18n.t('sidebar.newItem')}
      aria-label={i18n.t('sidebar.newItem')}
    >
      <Plus size={18} />
      <span class="hidden lg:block font-bold text-[10px] uppercase tracking-widest">{i18n.t('sidebar.newItem')}</span>
    </button>
  </div>
  
  <nav class="px-1 md:px-2 lg:px-4 space-x-1 md:space-x-0 md:space-y-1 flex flex-row md:flex-col overflow-x-auto md:overflow-x-visible md:overflow-y-auto custom-scrollbar shrink-0 md:flex-1 items-center md:items-stretch h-full md:h-auto">
    <button 
      onclick={() => activeFilter = 'all'}
      class="w-12 md:w-full h-11 rounded-xl flex items-center justify-center lg:justify-start lg:px-4 gap-0 lg:gap-4 transition-all shrink-0 {activeFilter === 'all' ? '' : 'opacity-40 hover:opacity-100 hover:bg-black/5'}"
      style={activeFilter === 'all' ? 'background-color: var(--md-primary-container); color: var(--md-on-primary-container);' : ''}
    >
      <Grid size={16} />
      <span class="hidden lg:block font-ui font-bold text-[9px] uppercase tracking-widest">{i18n.t('sidebar.all')}</span>
    </button>
    
    <button 
      onclick={() => activeFilter = 'favorites'}
      class="w-12 md:w-full h-11 rounded-xl flex items-center justify-center lg:justify-start lg:px-4 gap-0 lg:gap-4 transition-all shrink-0 {activeFilter === 'favorites' ? '' : 'opacity-40 hover:opacity-100 hover:bg-black/5'}"
      style={activeFilter === 'favorites' ? 'background-color: var(--md-primary-container); color: var(--md-on-primary-container);' : ''}
    >
      <Star size={16} fill={activeFilter === 'favorites' ? "currentColor" : "none"} />
      <span class="hidden lg:block font-ui font-bold text-[9px] uppercase tracking-widest">{i18n.t('sidebar.favorites')}</span>
    </button>

    <div class="w-px md:w-auto h-8 md:h-px bg-black/[0.1] dark:bg-white/[0.1] my-auto md:my-4 mx-2 shrink-0"></div>
    
    <div class="hidden lg:flex items-center justify-between px-4 mb-2 shrink-0">
      <p class="text-[8px] font-black uppercase tracking-[0.2em] opacity-20">{i18n.t('sidebar.folders')}</p>
      <button 
        onclick={onOpenNewFolder} 
        class="p-1 rounded-md hover:bg-black/5 dark:hover:bg-white/5 opacity-20 hover:opacity-100 transition-colors" 
        style="--hover-color: {userColor};" 
        onmouseenter={(e) => applyStyle(e, 'color', 'var(--hover-color)')} 
        onmouseleave={(e) => applyStyle(e, 'color', '')} 
        title={i18n.t('sidebar.newFolder')}
        aria-label={i18n.t('sidebar.newFolder')}
      >
        <Plus size={12} />
      </button>
    </div>
    
    {#each folders.filter(f => f.uid !== 'system-trash') as f}
      {@const SidebarIcon = iconMap[f.icon] || Folder}
      <button 
        onclick={() => activeFilter = `folder-${f.id}`}
        class="w-12 md:w-full h-11 rounded-xl flex items-center justify-center lg:justify-start lg:px-4 gap-0 lg:gap-4 transition-all shrink-0 {activeFilter === `folder-${f.id}` ? '' : 'opacity-40 hover:opacity-100 hover:bg-black/5'}"
        style={activeFilter === `folder-${f.id}` ? 'background-color: var(--md-primary-container); color: var(--md-on-primary-container);' : ''}
      >
        <SidebarIcon size={16} style="color: {f.color}" />
        <span class="hidden lg:block font-ui font-bold text-[9px] uppercase tracking-widest">{f.name}</span>
      </button>
    {/each}
  </nav>

  <div class="px-1 md:px-2 lg:px-4 md:mt-auto pt-0 md:pt-4 flex items-center shrink-0 border-l md:border-l-0 border-border ml-2 md:ml-0 pl-2 md:pl-0">
    {#if activeFilter === 'trash'}
      <button 
        onclick={onEmptyTrash}
        class="w-12 md:w-full h-11 rounded-xl flex items-center justify-center lg:justify-start lg:px-4 gap-0 lg:gap-4 transition-all bg-red-600 text-white shadow-elev-2 shadow-red-500/20 active:scale-95 group shrink-0"
      >
        <Trash2 size={16} class="group-hover:scale-110 transition-transform" />
        <span class="hidden lg:block font-ui font-bold text-[9px] uppercase tracking-widest">{i18n.t('sidebar.emptyTrash')}</span>
      </button>
    {:else}
      <button 
        onclick={() => activeFilter = 'trash'}
        class="w-12 md:w-full h-11 rounded-xl flex items-center justify-center lg:justify-start lg:px-4 gap-0 lg:gap-4 transition-all opacity-40 hover:opacity-100 hover:bg-black/5 shrink-0"
      >
        <Trash2 size={16} />
        <span class="hidden lg:block font-ui font-bold text-[9px] uppercase tracking-widest">{i18n.t('sidebar.trash')}</span>
      </button>
    {/if}
  </div>
</aside>
