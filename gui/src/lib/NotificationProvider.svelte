<script>
  import { globalState } from './state.svelte.js';
  import { X, CheckCircle2, AlertCircle, Info, ShieldAlert } from 'lucide-svelte';
  import { fly, fade } from 'svelte/transition';

  const icons = {
    success: { icon: CheckCircle2, color: 'text-green-500', bg: 'bg-green-500/10' },
    error: { icon: ShieldAlert, color: 'text-red-500', bg: 'bg-red-500/10' },
    warning: { icon: AlertCircle, color: 'text-orange-500', bg: 'bg-orange-500/10' },
    info: { icon: Info, color: 'text-[var(--md-primary)]', bg: 'bg-[var(--md-primary)]/10' }
  };
</script>

<div class="fixed bottom-12 left-1/2 -translate-x-1/2 z-[200] flex flex-col gap-3 items-center pointer-events-none w-full max-w-[400px] px-6">
  {#each globalState.notifications as n (n.id)}
    {@const IconData = icons[n.type]}
    {@const IconComp = IconData.icon}
    <div 
      in:fly={{ y: 40, duration: 600, opacity: 0 }} 
      out:fade={{ duration: 300 }}
      class="pointer-events-auto bg-[var(--md-surface-2)] backdrop-blur-2xl border border-border rounded-[28px] p-4 pr-6 shadow-elev-3 flex items-center gap-4 relative overflow-hidden group hover:scale-[1.02] transition-transform w-full"
    >
      <!-- Icono Dinámico (Izquierda) -->
      <div class="w-11 h-11 rounded-2xl flex items-center justify-center shrink-0 {IconData.bg}">
        <IconComp class={IconData.color} size={22} />
      </div>

      <!-- Texto (Alineado a la izquierda) -->
      <div class="flex-1 flex flex-col gap-0.5 text-left">
        <h5 class="text-[10px] font-black uppercase tracking-[0.2em] leading-tight opacity-90">{n.title}</h5>
        <p class="text-[11px] opacity-50 font-bold leading-tight">{n.message}</p>
      </div>

      <!-- Barra de estado (Inferior) -->
      <div class="absolute bottom-0 left-0 right-0 h-1 {IconData.color.replace('text-', 'bg-')} opacity-30"></div>
    </div>
  {/each}
</div>

<style>
  @keyframes progress {
    from { width: 100%; }
    to { width: 0%; }
  }
</style>
