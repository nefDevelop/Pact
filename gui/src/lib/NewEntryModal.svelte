<script>
  import { X, User, Lock, Save, Dice5, Eye, EyeOff, Folder, Palette, ChevronDown, ChevronUp, KeyRound, ClipboardPaste, Globe, Plus, AlertTriangle, Check, Circle, ShieldCheck, Loader2 } from 'lucide-svelte';
  import { globalState } from './state.svelte.js';
  import { encryptE2E } from './crypto_utils.js';
  import { focusTrap } from './focusTrap.js';
  import { i18n } from './i18n.svelte.js';
  import bip39 from './bip39_es.json';

  let { isOpen, onSave, onClose, onOpenNewFolder, folders = [], mode = 'normal', entry = null, isUnlocked } = $props();

  const isEditing = $derived(!!entry?.id);

  let title = $state('');
  let username = $state('');
  let password = $state('');
  let totpSeed = $state('');
  let website = $state('');
  let showPassword = $state(false);
  let showAdvanced = $state(false);
  let color = $state('#6750A4');
  let clipboardError = $state(false);
  let folderId = $state(null);
  let isSaving = $state(false);
  let removeTotp = $state(false);

  const colors = [
    '#F44336', '#E91E63', '#9C27B0', '#673AB7', '#3F51B5', '#2196F3', '#03A9F4', '#00BCD4',
    '#009688', '#4CAF50', '#8BC34A', '#CDDC39', '#FFEB3B', '#FFC107', '#FF9800', '#FF5722'
  ];

  const forbiddenPasswords = ['12345678', 'password', 'contraseña', 'Pact2024'];
  const symbols = ['!', '#', '$', '.', '_', '*'];

  function generatePassphrase() {
    const words = bip39.words;
    if (!words) return;
    let chosenWords = [
      words[Math.floor(Math.random() * words.length)],
      words[Math.floor(Math.random() * words.length)],
      words[Math.floor(Math.random() * words.length)]
    ];
    const capIdx = Math.floor(Math.random() * chosenWords.length);
    chosenWords[capIdx] = chosenWords[capIdx].charAt(0).toUpperCase() + chosenWords[capIdx].slice(1);
    const num = Math.floor(Math.random() * 900) + 100;
    const sym = symbols[Math.floor(Math.random() * symbols.length)];
    password = `${chosenWords[0]}${chosenWords[1]}${chosenWords[2]}${sym}${num}`;
    showPassword = true;
  }

  async function pasteSeed() {
    try {
      const text = await navigator.clipboard.readText();
      totpSeed = text.trim();
    } catch (err) {
      clipboardError = true;
      setTimeout(() => clipboardError = false, 3000);
    }
  }

  // Pre-rellena el formulario al abrir el modal: con los datos descifrados de
  // la entrada al editar, o vacío al crear una nueva.
  $effect(() => {
    if (!isOpen) return;
    if (entry) {
      title = entry.title || '';
      username = entry.username || '';
      password = entry.password || '';
      website = entry.website || '';
      folderId = entry.folder_id ?? null;
      totpSeed = '';
      showAdvanced = true;
    } else {
      title = ''; username = ''; password = ''; totpSeed = ''; website = '';
      folderId = null; showAdvanced = false;
    }
    removeTotp = false;
    clipboardError = false;
  });

  let isCommonPassword = $derived(forbiddenPasswords.includes(password.toLowerCase()));
  let isCriticallyWeak = $derived(isCommonPassword || (password.length > 0 && password.length < 6));
  let isSeedValid = $derived(!totpSeed || /^[A-Z2-7]+=*$/.test(totpSeed.toUpperCase().replace(/\s/g, '')));

  let requirements = $derived([
    { label: '8+ Caracteres', met: password.length >= 8 },
    { label: 'Mayúscula', met: /[A-Z]/.test(password) },
    { label: 'Número', met: /[0-9]/.test(password) },
    { label: 'Símbolo', met: /[!@#$%^&*(),.?":{}|<>_]/.test(password) }
  ]);

  let passwordStrength = $derived.by(() => {
    if (!password) return 0;
    if (isCriticallyWeak) return 0;
    let metCount = requirements.filter(r => r.met).length;
    if (password.length < 8) return 1;
    return Math.min(metCount, 4);
  });

  const strengthText = ['Crítica', 'Débil', 'Media', 'Fuerte', 'Muy Fuerte'];
  const strengthColors = ['bg-red-600', 'bg-orange-600', 'bg-yellow-500', 'bg-green-500', 'bg-blue-500'];

  async function handleSave() {
    if (!title || !username || !password || isCriticallyWeak || !isSeedValid || isSaving) return;
    isSaving = true;
    
    // Si no hay llave de cifrado, no podemos guardar de forma segura E2E
    if (!globalState.masterKey) {
      globalState.notify('Error de Seguridad', 'La bóveda está bloqueada. No se puede cifrar el dato.', 'error');
      isSaving = false;
      return;
    }

    try {
      const websiteUrl = website || (title.toLowerCase().includes('.') ? title : `${title.toLowerCase()}.com`);

      // CIFRADO E2E: Cada campo sensible se cifra por separado con la MasterKey
      const encryptedTitle = await encryptE2E(title, globalState.masterKey);
      const encryptedUrl = await encryptE2E(websiteUrl, globalState.masterKey);
      const encryptedUser = await encryptE2E(username, globalState.masterKey);
      const encryptedPass = await encryptE2E(password, globalState.masterKey);
      const encryptedTotp = totpSeed.trim()
        ? await encryptE2E(totpSeed.trim(), globalState.masterKey)
        : null;
      // "" indica al backend que elimine el TOTP; null mantiene el actual.
      const totpPayload = removeTotp ? '' : encryptedTotp;

      if (onSave) {
        const saved = await onSave({
          id: entry?.id,
          folder_id: folderId,
          title: encryptedTitle,
          url: encryptedUrl,
          username_site: encryptedUser,
          encrypted_pass: encryptedPass,
          // El seed TOTP se cifra E2E antes de salir del cliente.
          totp_seed: totpPayload,
          is_favorite: entry ? entry.is_favorite : false,
          // Al editar se conserva el icono existente; al crear aún no hay.
          icon_data: entry?.icon_data ?? null
        });
        // Si el guardado falló, mantenemos el modal abierto con los datos.
        if (saved === false) return;
        resetForm();
        onClose();
      }
    } catch (e) {
      console.error("Error al cifrar para guardar:", e);
      globalState.notify('Error', 'Fallo al cifrar los datos localmente', 'error');
    } finally {
      isSaving = false;
    }
  }

  function resetForm() {
    title = ''; username = ''; password = ''; totpSeed = ''; website = ''; showAdvanced = false; clipboardError = false; folderId = null; removeTotp = false;
  }
</script>

{#if isOpen}
<div class="fixed inset-0 z-[120] flex items-center justify-center p-6 lg:p-12">
  <button class="absolute inset-0 bg-black/60 backdrop-blur-md animate-in fade-in border-none w-full h-full cursor-default" onclick={onClose} aria-label="Cerrar modal"></button>

  <div use:focusTrap role="dialog" aria-modal="true" aria-label={isEditing ? 'Editar entrada' : 'Nueva entrada'} class="w-full max-w-[500px] bg-[var(--md-surface-2)] rounded-[40px] border border-border relative z-10 overflow-hidden flex flex-col transition-colors duration-500 ease-in-out max-h-[90vh] shadow-elev-3">
    
    <div class="px-8 pt-8 pb-4 flex justify-between items-center shrink-0">
      <div class="flex flex-col">
        <div class="flex items-center gap-3">
          <h3 class="text-2xl font-bold tracking-tight text-[var(--md-on-background)]">{isEditing ? i18n.t('entry.edit') : i18n.t('entry.new')}</h3>
          {#if !isEditing && mode === 'quick'}<span class="px-2 py-0.5 rounded-md bg-[var(--md-primary)]/10 text-[var(--md-primary)] text-[7px] font-black uppercase tracking-tighter border border-[var(--md-primary)]/20 animate-pulse">Rápida</span>{/if}
        </div>
        <p class="text-[10px] font-black uppercase tracking-widest text-[var(--md-on-surface-variant)]">{isEditing ? i18n.t('entry.editSubtitle') : i18n.t('entry.addSubtitle')}</p>
      </div>
      <button onclick={onClose} class="w-10 h-10 rounded-full flex items-center justify-center hover:bg-black/5 dark:hover:bg-white/5 transition-colors opacity-40 hover:opacity-100" aria-label="Cerrar">
        <X size={20} />
      </button>
    </div>

    <div class="flex-1 overflow-y-auto p-8 pt-2 space-y-6 custom-scrollbar">
      <div class="space-y-4">
        <div class="flex items-end gap-4">
          <div class="w-14 h-14 rounded-2xl border border-border flex items-center justify-center text-lg font-black text-white shrink-0 transition-all duration-300" style="background-color: {color}">
            {title ? title.substring(0, 2).toUpperCase() : '?'}
          </div>
          <div class="flex-1 space-y-1.5">
            <label for="e-title" class="text-[9px] font-black uppercase tracking-widest text-[var(--md-on-surface-variant)] px-2">{i18n.t('entry.service')}</label>
            <input id="e-title" bind:value={title} placeholder="Ej. GitHub" class="w-full h-11 bg-[var(--md-surface-variant)] rounded-xl px-4 outline-none font-bold text-sm border border-transparent focus:border-[var(--md-primary)]/30 text-[var(--md-on-background)] transition-all" />
          </div>
        </div>

        <div class="space-y-1.5">
          <label for="e-user" class="text-[9px] font-black uppercase tracking-widest text-[var(--md-on-surface-variant)] px-2 flex items-center gap-2"><User size={10}/> {i18n.t('entry.user')}</label>
          <input id="e-user" bind:value={username} placeholder="nombre@ejemplo.com" class="w-full h-12 bg-[var(--md-surface-variant)] rounded-xl px-5 outline-none text-sm border border-transparent focus:border-[var(--md-primary)]/30 text-[var(--md-on-background)] transition-all" />
        </div>

        <div class="space-y-3">
          <div class="flex justify-between items-center px-2">
            <label for="e-pass" class="text-[9px] font-black uppercase tracking-widest text-[var(--md-on-surface-variant)] flex items-center gap-2"><Lock size={10}/> {i18n.t('entry.password')}</label>
            {#if password}
              <span class="text-[8px] font-black uppercase tracking-tighter {isCriticallyWeak ? 'text-red-600 animate-pulse' : (passwordStrength >= 4 ? 'text-blue-500' : 'opacity-40')}">
                {strengthText[passwordStrength]}
              </span>
            {/if}
          </div>
          <div class="relative">
            <input id="e-pass" type={showPassword ? 'text' : 'password'} bind:value={password} placeholder="••••••••" class="w-full h-12 bg-[var(--md-surface-variant)] rounded-xl pl-5 pr-24 outline-none text-sm border border-transparent focus:border-[var(--md-primary)]/30 text-[var(--md-on-background)] {isCriticallyWeak ? 'ring-2 ring-red-500/20 border-red-500/50' : ''}" />
            <div class="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-1">
              <button onclick={generatePassphrase} class="p-2 rounded-lg hover:bg-[var(--md-primary)]/10 text-[var(--md-primary)] opacity-60 hover:opacity-100 transition-all" title={i18n.t('entry.generate')} aria-label={i18n.t('entry.generate')}><Dice5 size={16}/></button>
              <button onclick={() => showPassword = !showPassword} class="p-2 rounded-lg hover:bg-black/5 dark:hover:bg-white/5 opacity-30 hover:opacity-100 transition-all" aria-label={showPassword ? i18n.t('entry.hidePassword') : i18n.t('entry.showPassword')}>
                {#if showPassword} <EyeOff size={16} /> {:else} <Eye size={16} /> {/if}
              </button>
            </div>
          </div>

          {#if isCriticallyWeak}
            <div class="bg-red-500/5 border border-red-500/10 rounded-xl p-3 flex items-start gap-3 animate-in fade-in slide-in-from-top-1">
              <AlertTriangle size={14} class="text-red-500 shrink-0 mt-0.5" />
              <p class="text-[10px] opacity-70 leading-tight text-red-600 font-bold">{i18n.t('entry.weak')}</p>
            </div>
          {:else}
            <div class="flex gap-1.5 h-1 px-2">
              {#each Array(4) as _, i}
                <div class="flex-1 rounded-full transition-all {i < passwordStrength && password ? strengthColors[passwordStrength] : 'bg-[var(--md-surface-variant)]'}"></div>
              {/each}
            </div>
            <div class="grid grid-cols-4 gap-1 px-2">
              {#each requirements as req}
                <div class="flex flex-col items-center gap-1 {req.met ? 'opacity-100' : 'opacity-20'} transition-opacity">
                  {#if req.met}
                    <Check size={8} class="text-green-500" strokeWidth={4} />
                  {:else}
                    <Circle size={8} strokeWidth={3} />
                  {/if}
                  <span class="text-[7px] font-black uppercase tracking-tighter whitespace-nowrap">{req.label}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <div class="space-y-1.5">
          <div class="flex justify-between items-center px-2">
            <label for="e-2fa" class="text-[9px] font-black uppercase tracking-widest text-[var(--md-on-surface-variant)] flex items-center gap-2"><KeyRound size={10}/> {i18n.t('entry.totpLabel')}</label>
            {#if totpSeed && !isSeedValid}
              <span class="text-[8px] font-black uppercase tracking-tighter text-red-600 animate-pulse">{i18n.t('entry.totpInvalid')}</span>
            {:else if totpSeed && isSeedValid}
              <div class="flex items-center gap-1 text-green-500">
                <ShieldCheck size={10} />
                <span class="text-[8px] font-black uppercase tracking-tighter opacity-60">{i18n.t('entry.totpValid')}</span>
              </div>
            {/if}
          </div>
          {#if isEditing && entry?.has_totp}
            <div class="flex items-center justify-between px-3 py-2 rounded-xl bg-[var(--md-surface-variant)]">
              <span class="text-[9px] font-black uppercase tracking-widest {removeTotp ? 'text-red-500' : 'text-green-600'}">
                {removeTotp ? i18n.t('entry.totpWillRemove') : i18n.t('entry.totpOn')}
              </span>
              <button onclick={() => { removeTotp = !removeTotp; if (removeTotp) totpSeed = ''; }} class="text-[9px] font-black uppercase tracking-widest {removeTotp ? 'text-green-600' : 'text-red-500'} hover:opacity-70 transition-opacity">
                {removeTotp ? i18n.t('entry.totpKeep') : i18n.t('entry.totpRemove')}
              </button>
            </div>
          {/if}
          <div class="relative">
            <input id="e-2fa" bind:value={totpSeed} placeholder="JBSWY3DPEHPK3PXP" class="w-full h-12 bg-[var(--md-surface-variant)] rounded-xl pl-5 pr-12 outline-none text-xs font-mono border border-transparent focus:border-[var(--md-primary)]/30 uppercase {totpSeed && !isSeedValid ? 'ring-2 ring-red-500/20 border-red-500/50' : ''}" />
            <button onclick={pasteSeed} class="absolute right-2 top-1/2 -translate-y-1/2 p-2 rounded-lg hover:bg-[var(--md-primary)]/10 text-[var(--md-primary)] opacity-60 hover:opacity-100 transition-all" title={i18n.t('entry.paste')} aria-label={i18n.t('entry.paste')}>
              <ClipboardPaste size={16} />
            </button>
          </div>
          {#if totpSeed && !isSeedValid}
            <div class="px-2 flex items-start gap-2 animate-in fade-in slide-in-from-top-1">
              <AlertTriangle size={10} class="text-red-500 mt-0.5" />
              <p class="text-[8px] text-red-500 font-bold leading-tight opacity-70">
                Solo letras A-Z y números 2-7. Sin espacios.
              </p>
            </div>
          {/if}
          {#if clipboardError}
            <p class="text-[8px] text-red-500 font-bold px-2 animate-in fade-in slide-in-from-top-1">
              {i18n.t('entry.pasteHint')}
            </p>
          {/if}
        </div>
      </div>

      <div class="pt-2">
        <button onclick={() => showAdvanced = !showAdvanced} class="flex items-center gap-2 text-[9px] font-black uppercase tracking-widest opacity-20 hover:opacity-100 transition-opacity mb-4">
          {#if showAdvanced}<ChevronUp size={12}/>{:else}<ChevronDown size={12}/>{/if}
          {i18n.t('entry.advanced')}
        </button>

        {#if showAdvanced}
          <div class="space-y-6 animate-in slide-in-from-top-2">
            <div class="space-y-1.5">
              <label for="e-web" class="text-[9px] font-black uppercase tracking-widest text-[var(--md-on-surface-variant)] px-2 flex items-center gap-2"><Globe size={10}/> {i18n.t('entry.website')}</label>
              <input id="e-web" bind:value={website} placeholder="https://ejemplo.com" class="w-full h-12 bg-[var(--md-surface-variant)] rounded-xl px-5 outline-none text-sm border border-transparent focus:border-[var(--md-primary)]/30" />
            </div>

            <div class="space-y-3">
              <span class="text-[9px] font-black uppercase tracking-widest text-[var(--md-on-surface-variant)] px-2 flex items-center gap-2"><Palette size={10}/> {i18n.t('entry.color')}</span>
              <div class="flex flex-wrap gap-3 px-2">
                {#each colors as c}
                  <button aria-label="Elegir color {c}" onclick={() => color = c} class="w-7 h-7 rounded-full transition-all hover:scale-110 {color === c ? 'ring-2 ring-[var(--md-primary)] ring-offset-4 dark:ring-offset-[#16151a]' : ''}" style="background-color: {c}"></button>
                {/each}
              </div>
            </div>

            <div class="space-y-3">
              <div class="flex justify-between items-center px-2">
                <span class="text-[9px] font-black uppercase tracking-widest text-[var(--md-on-surface-variant)] flex items-center gap-2"><Folder size={10}/> {i18n.t('entry.category')}</span>
                <button onclick={onOpenNewFolder} class="text-[8px] font-black uppercase tracking-widest text-[var(--md-primary)] hover:opacity-100 transition-opacity flex items-center gap-1">
                  <Plus size={10}/> {i18n.t('entry.newCategory')}
                </button>
              </div>

              <div class="grid grid-cols-2 gap-2">
                <button onclick={() => folderId = null} class="h-10 rounded-xl border transition-all text-[9px] font-bold uppercase tracking-widest {folderId === null ? 'bg-[var(--md-primary)] text-[var(--md-on-primary)] border-[var(--md-primary)] shadow-elev-1 ' : 'border-border opacity-40 hover:opacity-100'}">
                  {i18n.t('common.none')}
                </button>
                {#each folders.filter(f => f.uid !== 'system-trash') as f}
                  <button onclick={() => folderId = f.id} class="h-10 rounded-xl border transition-all text-[9px] font-bold uppercase tracking-widest {folderId === f.id ? 'bg-[var(--md-primary)] text-[var(--md-on-primary)] border-[var(--md-primary)] shadow-elev-1 ' : 'border-border opacity-40 hover:opacity-100'}">
                    {f.name}
                  </button>
                {/each}
              </div>
            </div>
          </div>
        {/if}
      </div>
    </div>

    <div class="p-8 bg-[var(--md-surface-variant)] border-t border-border flex justify-end gap-4 items-center shrink-0">
      <button onclick={onClose} class="px-6 h-12 rounded-full text-[10px] font-black uppercase tracking-widest opacity-40 hover:opacity-100 transition-all">{i18n.t('common.cancel')}</button>
      <button onclick={handleSave} disabled={!title || !username || !password || isCriticallyWeak || !isSeedValid || isSaving} class="px-10 h-14 rounded-full bg-[var(--md-primary)] text-[var(--md-on-primary)] font-black text-[11px] uppercase tracking-[0.2em] transition-all flex items-center gap-3 active:scale-95 disabled:opacity-20 disabled:grayscale shadow-elev-2">
        {#if isSaving}
          <Loader2 size={18} class="animate-spin" /> {i18n.t('common.saving')}
        {:else}
          <ShieldCheck size={18} /> 
          {isEditing ? i18n.t('entry.saveChanges') : (mode === 'quick' ? i18n.t('entry.addNow') : i18n.t('entry.saveEntry'))}
        {/if}
      </button>
    </div>
  </div>
</div>
{/if}

<style>
  .custom-scrollbar::-webkit-scrollbar { width: 4px; }
  .custom-scrollbar::-webkit-scrollbar-track { background: transparent; }
  .custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(0, 0, 0, 0.05); border-radius: 10px; }
  :global(.dark) .custom-scrollbar::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.05); }
</style>
