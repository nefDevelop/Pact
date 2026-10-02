<script>
  import { X, Palette, Lock, Eye, EyeOff, ShieldCheck, Check, Circle, AlertTriangle, Dice5, RefreshCw } from 'lucide-svelte';
  import bip39 from './bip39_es.json';
  import { globalState } from './state.svelte.js';
  import { api } from './api.js';
  import { deriveKeys } from './crypto_utils.js';
  import { focusTrap } from './focusTrap.js';
  
  let { isOpen, onSave, onClose, users = [] } = $props();
  
  let name = $state('');
  let password = $state('');
  let showPassword = $state(false);
  let color = $state('');
  let seed = $state(Math.random().toString(36).substring(7));

  let colors = $state([
    '#F44336', '#E91E63', '#9C27B0', '#673AB7', '#3F51B5', '#2196F3', '#03A9F4', '#00BCD4',
    '#009688', '#4CAF50', '#8BC34A', '#CDDC39', '#FFEB3B', '#FFC107', '#FF9800', '#FF5722',
    '#795548', '#9E9E9E', '#607D8B', '#E64A19', '#1976D2', '#388E3C', '#FBC02D', '#C2185B',
    '#5D4037', '#455A64', '#512DA8', '#00796B', '#AFB42B', '#FFA000', '#F57C00', '#D32F2F'
  ]);

  let takenColors = $derived(users.map(u => (u.color || '').toLowerCase()).filter(c => c.startsWith('#')));
  
  let availableColors = $derived(
    colors.filter(c => isColorDistinct(c.toLowerCase(), 25)).slice(0, 18)
  );

  function hexToRgb(hex) {
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return [r, g, b];
  }

  function rgbToLab(r, g, b) {
    let [x, y, z] = [r / 255, g / 255, b / 255].map(v => 
      v > 0.04045 ? Math.pow((v + 0.055) / 1.055, 2.4) : v / 12.92
    );
    [x, y, z] = [
      (x * 0.4124 + y * 0.3576 + z * 0.1805) * 100 / 95.047,
      (x * 0.2126 + y * 0.7152 + z * 0.1089) * 100 / 100.000,
      (x * 0.0193 + y * 0.1192 + z * 0.9505) * 100 / 108.883
    ].map(v => v > 0.008856 ? Math.pow(v, 1/3) : (7.787 * v) + (16 / 116));

    return [(116 * y) - 16, 500 * (x - y), 200 * (y - z)];
  }

  function getDeltaE(hex1, hex2) {
    const lab1 = rgbToLab(...hexToRgb(hex1));
    const lab2 = rgbToLab(...hexToRgb(hex2));
    return Math.sqrt(
      Math.pow(lab1[0] - lab2[0], 2) +
      Math.pow(lab1[1] - lab2[1], 2) +
      Math.pow(lab1[2] - lab2[2], 2)
    );
  }

  function isColorDistinct(candidate, threshold = 18) {
    if (takenColors.length === 0) return true;
    for (const taken of takenColors) {
      if (getDeltaE(candidate, taken) < threshold) return false;
    }
    return true;
  }

  $effect(() => {
    if (isOpen) {
      const firstAvailable = availableColors[0];
      color = firstAvailable || colors[0];
    }
  });

  function refreshColors() {
    colors = [...colors].sort(() => Math.random() - 0.5);
    const firstAvailable = availableColors[0];
    if (firstAvailable) color = firstAvailable;
  }

  $effect(() => {
    if (name.trim()) {
      seed = name.trim();
    }
  });

  const forbiddenPasswords = [
    '12345678', 'password', 'contraseña', 'Pact2024', 'qwertyuiop', 
    'admin123', '123456789', 'password123', 'Mypassword123!', 'Admin123!'
  ];

  const symbols = ['!', '#', '$', '.', '_', '*'];

  function generatePassphrase() {
    const words = bip39.words;
    let chosenWords = [
      words[Math.floor(Math.random() * words.length)],
      words[Math.floor(Math.random() * words.length)],
      words[Math.floor(Math.random() * words.length)]
    ];
    
    const capIdx = Math.floor(Math.random() * chosenWords.length);
    chosenWords[capIdx] = chosenWords[capIdx].charAt(0).toUpperCase() + chosenWords[capIdx].slice(1);
    
    const num = Math.floor(Math.random() * 900) + 100;
    const sym = symbols[Math.floor(Math.random() * symbols.length)];
    
    // Aleatorizamos la posición del símbolo para romper patrones predecibles
    const symPos = Math.floor(Math.random() * 3);
    if (symPos === 0) password = `${chosenWords[0]}${sym}${chosenWords[1]}${chosenWords[2]}${num}`;
    else if (symPos === 1) password = `${chosenWords[0]}${chosenWords[1]}${sym}${chosenWords[2]}${num}`;
    else password = `${chosenWords[0]}${chosenWords[1]}${chosenWords[2]}${sym}${num}`;

    showPassword = true;
  }

  let isCommonPassword = $derived(forbiddenPasswords.includes(password.toLowerCase()));
  let isRepeatingChars = $derived(/(.)\1{3,}/.test(password)); 
  let isSequential = $derived(/123|abc|asd|qwe/.test(password.toLowerCase()));
  let isTooSimple = $derived(password.length > 0 && password.length < 6 && (/^\d+$/.test(password) || /^[a-z]+$/i.test(password)));

  let isCriticallyWeak = $derived(
    isCommonPassword || 
    isTooSimple ||
    ((isRepeatingChars || isSequential) && password.length < 12)
  );

  let requirements = $derived([
    { label: 'Mínimo 12 caracteres', met: password.length >= 12 },
    { label: 'Una mayúscula', met: /[A-Z]/.test(password) },
    { label: 'Un número', met: /[0-9]/.test(password) },
    { label: 'Símbolo (!@#$...)', met: /[!@#$%^&*(),.?":{}|<>_]/.test(password) }
  ]);

  let passwordStrength = $derived.by(() => {
    if (isCriticallyWeak || password.length === 0) return 0;
    const metCount = requirements.filter(r => r.met).length;
    if (password.length < 12) return Math.min(metCount, 3);
    if (metCount === 4) return 4;
    return Math.min(metCount, 3);
  });

  const strengthText = ['Vulnerabilidad Crítica', 'Muy Corta', 'Débil', 'Media', 'Seguridad Alta'];
  const strengthColors = ['bg-red-600', 'bg-orange-600', 'bg-orange-400', 'bg-yellow-500', 'bg-blue-500'];

  let isRegistering = $state(false);

  async function handleSave() {
    if (!name || passwordStrength < 4 || isCriticallyWeak) return;
    
    isRegistering = true;
    try {
      const saltBytes = crypto.getRandomValues(new Uint8Array(16));
      const saltHex = Array.from(saltBytes).map(b => b.toString(16).padStart(2, '0')).join('');

      const { masterKey, authKey } = await deriveKeys(password, saltHex);

      await api.post('/auth/register', {
        username: name,
        master_password: authKey,
        salt: saltHex,
        color: color
      });

      globalState.masterKey = masterKey;

      onSave({
        name,
        password: '',
        color,
        avatar: `https://api.dicebear.com/9.x/notionists/svg?seed=${seed}`,
        status: 'Protegida'
      });
      
      resetForm();
      onClose();
    } catch (error) {
      console.error('Error de registro:', error);
      globalState.notify('Error', 'No se pudo crear el usuario.', 'error');
    } finally {
      isRegistering = false;
    }
  }

  function resetForm() {
    name = ''; 
    password = ''; 
    seed = Math.random().toString(36).substring(7);
  }

  function generateNewAvatar() {
    seed = Math.random().toString(36).substring(7);
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

{#if isOpen}
<div class="fixed inset-0 z-[100] flex items-center justify-center p-6 lg:p-12">
  <div 
    class="absolute inset-0 bg-black/60 backdrop-blur-md animate-in fade-in "
    onclick={onClose}
    role="presentation"
  ></div>

  <div use:focusTrap role="dialog" aria-modal="true" aria-label="Nuevo usuario" class="w-full max-w-[500px] bg-[var(--md-surface)] rounded-[40px] border border-black/[0.1] dark:border-white/[0.1] relative z-10 overflow-hidden transition-colors duration-500 ease-in-out shadow-2xl">
    
    <div class="px-8 pt-8 pb-4 flex justify-between items-center transition-colors duration-500 ease-in-out">
      <div class="flex flex-col">
        <h3 class="text-2xl font-bold tracking-tight text-[var(--md-on-background)]">Nuevo Usuario</h3>
        <p class="text-[10px] font-black uppercase tracking-widest opacity-30 text-[var(--md-on-background)]">Configurar identidad personal</p>
      </div>
      <button 
        onclick={onClose}
        class="w-10 h-10 rounded-full flex items-center justify-center hover:bg-black/5 dark:hover:bg-white/5 transition-colors opacity-40 hover:opacity-100"
        aria-label="Cerrar"
      >
        <X size={20} />
      </button>
    </div>

    <div class="p-8 space-y-8">
      <!-- Identity Info -->
      <div class="flex items-center gap-6">
        <div class="relative group">
          <div class="w-24 h-24 rounded-full p-1 flex items-center justify-center overflow-hidden bg-[var(--md-surface-variant)] shadow-inner">
            <img 
              src={`https://api.dicebear.com/9.x/notionists/svg?seed=${seed}`} 
              alt="Avatar preview" 
              class="w-full h-full rounded-full object-cover bg-black/5"
              onerror={handleAvatarError}
            />
            <div 
              class="w-full h-full rounded-full items-center justify-center text-white text-3xl font-bold transition-colors hidden"
              style="background-color: {color}"
            >
              {name ? name.substring(0, 1).toUpperCase() : '?'}
            </div>
          </div>
          <button 
            onclick={generateNewAvatar}
            class="absolute -bottom-1 -right-1 w-8 h-8 rounded-full bg-white dark:bg-[#2b2930] border border-black/10 dark:border-white/10 flex items-center justify-center hover:scale-110 active:scale-95 transition-all shadow-sm"
            aria-label="Generar nuevo avatar"
          >
            <Dice5 size={14} />
          </button>
        </div>
        <div class="flex-1 flex flex-col gap-2">
          <label for="identity-alias" class="text-[10px] font-black uppercase tracking-widest opacity-30 px-2 text-[var(--md-on-background)]">Alias de Usuario</label>
          <input 
            id="identity-alias"
            type="text" 
            bind:value={name}
            placeholder="Ej. Neftali"
            class="w-full h-12 bg-[var(--md-surface-variant)] rounded-2xl px-5 focus:outline-none focus:ring-2 focus:ring-[#6750A4]/20 border border-transparent focus:border-[#6750A4]/10 text-sm font-bold text-[var(--md-on-background)] transition-all duration-300"
          />
        </div>
      </div>

      <!-- Master Password Section -->
      <div class="space-y-4">
        <div class="flex justify-between items-center px-2">
          <label for="master-pass" class="flex items-center gap-2 text-[10px] font-black uppercase tracking-widest opacity-30 text-[var(--md-on-background)]">
            <Lock size={10} /> Contraseña Maestra
          </label>
          {#if password}
            <span class="text-[9px] font-black uppercase tracking-tighter {isCriticallyWeak ? 'text-red-600 animate-pulse' : (passwordStrength >= 4 ? 'text-blue-500 font-black' : 'opacity-40')}">
              {strengthText[passwordStrength]}
            </span>
          {/if}
        </div>
        
        <div class="relative">
          <input 
            id="master-pass"
            type={showPassword ? 'text' : 'password'}
            bind:value={password}
            placeholder="Introduce una clave fuerte..."
            class="w-full h-14 bg-[var(--md-surface-variant)] rounded-2xl pl-6 pr-24 focus:outline-none focus:ring-2 focus:ring-[#6750A4]/20 border border-transparent focus:border-[#6750A4]/10 text-sm {isCriticallyWeak ? 'ring-2 ring-red-500/20 border-red-500/50' : ''}"
          />
          <div class="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-1">
            <button 
              onclick={generatePassphrase}
              class="p-2.5 rounded-xl hover:bg-[#6750A4]/10 text-[#6750A4] opacity-60 hover:opacity-100 transition-all"
              title="Generar frase segura"
              aria-label="Generar frase segura"
            >
              <Dice5 size={18} />
            </button>
            <button 
              onclick={() => showPassword = !showPassword}
              class="p-2.5 rounded-xl hover:bg-black/5 dark:hover:bg-white/5 opacity-30 hover:opacity-100 transition-all"
              aria-label={showPassword ? 'Ocultar contraseña' : 'Mostrar contraseña'}
            >
              {#if showPassword} <EyeOff size={18} /> {:else} <Eye size={18} /> {/if}
            </button>
          </div>
        </div>

        {#if isCriticallyWeak}
          <div class="bg-red-500/5 border border-red-500/10 rounded-2xl p-4 flex items-start gap-3 animate-in fade-in slide-in-from-top-2 ">
            <AlertTriangle size={16} class="text-red-500 shrink-0 mt-0.5" />
            <div class="flex flex-col">
              <span class="text-[10px] font-bold text-red-500 uppercase tracking-wider">Vulnerabilidad Crítica</span>
              <p class="text-[11px] opacity-60 leading-relaxed mt-1">Esta combinación es extremadamente fácil de predecir. Por favor, utiliza una clave más compleja.</p>
            </div>
          </div>
        {:else}
          <div class="grid grid-cols-2 gap-y-2 px-2">
            {#each requirements as req}
              <div class="flex items-center gap-2 transition-all  {req.met ? 'opacity-100' : 'opacity-20'}">
                {#if req.met}
                  <div class="w-4 h-4 rounded-full bg-green-500/10 flex items-center justify-center text-green-500">
                    <Check size={10} strokeWidth={4} />
                  </div>
                {:else}
                  <Circle size={10} strokeWidth={3} />
                {/if}
                <span class="text-[9px] font-bold uppercase tracking-wider">{req.label}</span>
              </div>
            {/each}
          </div>
        {/if}

        <div class="flex gap-1 h-1 px-2">
          {#each Array(4) as _, i}
            <div class="flex-1 rounded-full transition-all  {i < passwordStrength && password && !isCriticallyWeak ? strengthColors[passwordStrength] : (isCriticallyWeak && i === 0 ? strengthColors[0] : 'bg-black/5 dark:bg-white/5')}"></div>
          {/each}
        </div>
      </div>

      <!-- Visual Customization -->
      <div class="space-y-4">
        <div class="flex justify-between items-center px-2">
          <span class="flex items-center gap-2 text-[10px] font-black uppercase tracking-widest opacity-30">
            <Palette size={10} /> Identificador Visual
          </span>
          <button 
            onclick={refreshColors}
            class="p-1.5 rounded-lg hover:bg-black/5 dark:hover:bg-white/5 opacity-30 hover:opacity-100 transition-all group/refresh"
            aria-label="Cambiar paleta de colores"
          >
            <RefreshCw size={12} class="group-active/refresh:rotate-180 transition-transform " />
          </button>
        </div>
        <div class="px-2">
          <div class="grid grid-cols-9 gap-2">
            {#each availableColors as c}
              <button 
                onclick={() => (color = c)}
                class="w-7 h-7 rounded-full border-2 transition-all flex items-center justify-center hover:scale-110 cursor-pointer {color === c ? 'scale-110' : 'border-transparent'}"
                style="background-color: {c}; border-color: {color === c ? c : 'transparent'}; box-shadow: {color === c ? `0 0 0 4px ${c}33` : 'none'};"
                aria-label="Elegir color {c}"
              >
                {#if color === c}
                  <Check size={12} class="text-white" />
                {/if}
              </button>
            {/each}
          </div>
        </div>
      </div>
    </div>

    <!-- Actions -->
    <div class="p-8 bg-[var(--md-surface-variant)] border-t border-black/[0.03] dark:border-white/[0.03] flex justify-end gap-4 items-center">
      {#if isRegistering}
        <div class="flex items-center gap-4 pr-4 animate-in fade-in slide-in-from-right-4">
          <div class="flex flex-col items-end">
            <span class="text-[10px] font-black uppercase tracking-widest text-[#6750A4]">Cifrado de Grado Militar</span>
            <span class="text-[9px] opacity-40 font-bold uppercase">Procesando Argon2id...</span>
          </div>
          <div class="w-10 h-10 rounded-full border-4 border-[#6750A4]/10 border-t-[#6750A4] animate-spin"></div>
        </div>
      {:else}
        <button 
          onclick={onClose}
          class="px-6 h-12 rounded-full text-[10px] font-black uppercase tracking-widest opacity-40 hover:opacity-100 transition-all"
        >
          Cancelar
        </button>
        <button 
          onclick={handleSave}
          disabled={!name || passwordStrength < 4 || isCriticallyWeak}
          class="px-10 h-14 rounded-full text-white font-bold text-[11px] uppercase tracking-[0.2em] transition-all flex items-center gap-3 active:scale-95 disabled:opacity-20 disabled:grayscale shadow-lg shadow-black/10"
          style="background-color: {color}"
        >
          <ShieldCheck size={18} />
          Crear Identidad
        </button>
      {/if}
    </div>
  </div>
</div>
{/if}
