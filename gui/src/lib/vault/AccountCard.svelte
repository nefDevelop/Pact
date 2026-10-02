<script>
  import { User, Key, Copy, Eye, Star, ChevronDown, RefreshCw, Pencil, Trash2 } from 'lucide-svelte';
  import { globalState } from '../state.svelte.js';
  import { i18n } from '../i18n.svelte.js';

  let { 
    item, 
    userColor, 
    expanded = false, 
    onToggleExpanded,
    isFavorite, 
    isDeleted,
    totpProgress,
    totpSeconds,
    onToggleFavorite,
    onTogglePassword,
    onRestore,
    onEdit,
    onDelete,
    isVisible = false,
    iconMap,
    loadedIcons,
    onIconLoad,
    onIconError,
    getIconUrl
  } = $props();

  const FolderIcon = $derived(iconMap[item.folder_icon] || User);

  async function copyToClipboard(value, label) {
    if (!value) return;
    try {
      await navigator.clipboard.writeText(value);
      globalState.notify('Copiado', i18n.t('common.copied', { label }), 'success', 2000);
    } catch (e) {
      globalState.notify('Error', i18n.t('common.copyError'), 'error');
    }
  }
</script>

<div 
  class="bg-[var(--md-surface)] rounded-[32px] p-6 border border-border flex flex-col gap-4 animate-in slide-in-from-bottom-4 text-left w-full transition-[background-color,border-color,opacity]"
  style="--hover-border: {userColor}1a; {expanded ? `background-color: ${userColor}0d; border-color: ${userColor}1a;` : ''}"
  onmouseenter={(e) => !expanded && (e.currentTarget.style.borderColor = 'var(--hover-border)')}
  onmouseleave={(e) => !expanded && (e.currentTarget.style.borderColor = '')}
  role="article"
>
  <div class="flex items-center justify-between w-full">
    <div 
      role="button" 
      tabindex="0" 
      onclick={onToggleExpanded} 
      onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && onToggleExpanded()}
      class="flex items-center gap-4 cursor-pointer outline-none flex-1 min-w-0"
    >
      <div class="w-12 h-12 rounded-2xl flex items-center justify-center text-white font-bold shrink-0 shadow-elev-1 relative overflow-hidden" style="background-color: {item.icon_data || loadedIcons[item.id] ? 'transparent' : item.color}">
        {#if item.icon_data}
          <img src={item.icon_data} alt="" class="absolute inset-0 w-full h-full object-cover" />
        {:else}
          <img 
            src={getIconUrl(item)} 
            alt="" 
            class="absolute inset-0 w-full h-full object-cover opacity-0 transition-opacity"
            onload={(e) => onIconLoad(item, e)}
            onerror={(e) => onIconError(item, e)}
          />
        {/if}
        <span class="relative z-10 transition-opacity" class:opacity-0={item.icon_data || loadedIcons[item.id]}>{item.icon}</span>
      </div>
      <div class="flex flex-col flex-1 min-w-0">
        <div class="flex items-center gap-2">
          <span class="font-display font-black text-sm truncate">{item.title}</span>
          {#if !isDeleted}
            <button 
              onclick={(e) => { e.stopPropagation(); onToggleFavorite(item.id); }}
              class="p-1 rounded-lg transition-[background-color,border-color,opacity] {isFavorite ? 'text-yellow-500' : 'opacity-5 hover:opacity-100 hover:bg-black/5 dark:hover:bg-white/5'}"
              title={isFavorite ? "Quitar de favoritos" : "Añadir a favoritos"}
            >
              <Star size={14} fill={isFavorite ? "currentColor" : "none"} strokeWidth={2} />
            </button>
          {/if}
        </div>
        <div class="flex items-center gap-2">
          {#if item.folder_id && item.folder_id > 1}
            <FolderIcon size={10} class="opacity-30" />
          {/if}
          <span class="text-[10px] opacity-30 font-medium truncate">{item.website}</span>
        </div>
      </div>
    </div>
    
    <div class="flex items-center gap-1">
      {#if isDeleted}
        <button 
          onclick={(e) => { e.stopPropagation(); onRestore(item.id); }}
          class="p-2.5 rounded-xl text-green-500 bg-green-500/10 hover:bg-green-500/20 transition-[background-color,border-color,opacity]"
          title="Restaurar"
        >
          <RefreshCw size={18} />
        </button>
      {:else}
        <button
          onclick={(e) => { e.stopPropagation(); onEdit?.(item); }}
          class="p-2.5 rounded-xl opacity-20 hover:opacity-100 hover:bg-black/5 dark:hover:bg-white/5 transition-[background-color,border-color,opacity]"
          title="Editar"
          aria-label="Editar entrada"
        >
          <Pencil size={16} />
        </button>
        <button
          onclick={(e) => { e.stopPropagation(); onDelete?.(item.id); }}
          class="p-2.5 rounded-xl opacity-20 hover:opacity-100 hover:bg-red-500/10 hover:text-red-500 transition-[background-color,border-color,opacity]"
          title="Eliminar"
          aria-label="Eliminar entrada"
        >
          <Trash2 size={16} />
        </button>
      {/if}
      <button 
        onclick={onToggleExpanded}
        class="p-2.5 rounded-xl opacity-20 hover:opacity-100 hover:bg-black/5 dark:hover:bg-white/5 transition-[background-color,border-color,opacity]"
      >
        <ChevronDown size={18} class="transition-transform {expanded ? 'rotate-180' : ''}" />
      </button>
    </div>
  </div>
  
  {#if expanded}
    <div 
      role="presentation"
      class="grid grid-cols-1 gap-2 animate-in slide-in-from-top-2 w-full" 
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <div class="h-12 bg-[var(--md-surface-variant)] transition-colors duration-500 rounded-2xl px-4 flex items-center justify-between group">
        <div class="flex items-center gap-3 overflow-hidden">
          <User size={14} class="opacity-20 shrink-0" />
          <span class="text-xs font-medium truncate">{item.username}</span>
        </div>
        <button onclick={() => copyToClipboard(item.username, i18n.t('common.copyUser'))} class="p-2 opacity-20 hover:opacity-100 transition-opacity" title={i18n.t('common.copyUser')} aria-label={i18n.t('common.copyUser')}><Copy size={14}/></button>
      </div>
      <div class="h-12 bg-[var(--md-surface-variant)] transition-colors duration-500 rounded-2xl px-4 flex items-center justify-between group">
        <div class="flex items-center gap-3 overflow-hidden">
          <Key size={14} class="opacity-20 shrink-0" />
          <div class="flex gap-1.5">
            {#if isVisible} <span class="text-xs font-mono font-bold tracking-wider">{item.password}</span>
            {:else} {#each Array(8) as _}<div class="w-1.5 h-1.5 rounded-full bg-current opacity-20"></div>{/each} {/if}
          </div>
        </div>
        <div class="flex gap-1">
          <button onclick={() => onTogglePassword(item.id)} class="p-2 opacity-20 hover:opacity-100 transition-opacity" title={i18n.t('entry.showPassword')} aria-label={i18n.t('entry.showPassword')}><Eye size={14}/></button>
          <button onclick={() => copyToClipboard(item.password, i18n.t('common.copyPass'))} class="p-2 opacity-20 hover:opacity-100 transition-opacity" title={i18n.t('common.copyPass')} aria-label={i18n.t('common.copyPass')}><Copy size={14}/></button>
        </div>
      </div>
      
      {#if item.totp}
        <div class="h-12 bg-[var(--md-surface-variant)] transition-colors duration-500 rounded-2xl px-4 flex items-center justify-between group relative overflow-hidden">
          <div class="flex items-center gap-3 relative z-10">
            <span class="text-xs font-mono font-black tracking-[0.3em] opacity-60">{item.totp}</span>
          </div>
          <div class="flex items-center gap-3 relative z-10">
            <span class="text-[9px] font-black opacity-20 font-mono">{totpSeconds}s</span>
            <button onclick={() => copyToClipboard(item.totp, i18n.t('common.copyTotp'))} class="p-2 opacity-20 hover:opacity-100 transition-opacity" title={i18n.t('common.copyTotp')} aria-label={i18n.t('common.copyTotp')}><Copy size={14}/></button>
          </div>
          <div class="absolute bottom-0 left-0 h-[2px] transition-all" style="width: {totpProgress}%; background-color: {userColor}; opacity: 0.3;"></div>
        </div>
      {/if}
    </div>
  {/if}
</div>
