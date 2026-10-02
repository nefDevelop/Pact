/**
 * Modo claro/oscuro de la aplicación.
 *
 * Antes esta inicialización vivía dentro de `Footer.svelte`, un componente del
 * pie de página: la clase `dark` y la paleta se aplicaban *después* del primer
 * pintado, así que la app arrancaba con los colores claros de `:root` y luego
 * saltaba a oscuro. En el navegador el salto era imperceptible; en el AppImage
 * (WebKitGTK, CSP estricta y protocolo propio) el frame claro se veía.
 *
 * Ahora el tema se resuelve antes de montar la app, y `Footer` solo lo cambia.
 */

import { globalState } from './state.svelte.js';
import { applyM3Palette } from './theme.js';
import { safeStorage } from './crypto_utils.js';

const THEME_KEY = 'pact_theme';

/**
 * Decide el tema inicial: preferencia guardada, y si no hay, la del sistema.
 * @returns {boolean} `true` para oscuro
 */
export function resolveInitialTheme() {
    if (typeof window === 'undefined') return false;

    try {
        const saved = safeStorage.getItem(THEME_KEY);
        if (saved === 'dark') return true;
        if (saved === 'light') return false;
    } catch {
        // localStorage bloqueado: seguimos con la preferencia del sistema.
    }

    return !!window.matchMedia?.('(prefers-color-scheme: dark)')?.matches;
}

/**
 * Aplica tema y paleta al documento. Es idempotente y barata.
 *
 * Se llama al arrancar y cada vez que cambia el tema o el color del usuario.
 * @param {string} [hex] color semilla; por defecto `globalState.userColor`
 */
export function applyTheme(hex) {
    if (typeof document === 'undefined') return;

    const root = document.documentElement;
    root.classList.toggle('dark', globalState.isDark);
    // `color-scheme` le dice al webview que use el tema nativo correspondiente
    // (scrollbars, formularios, caret del cursor).
    root.style.colorScheme = globalState.isDark ? 'dark' : 'light';
    applyM3Palette(hex || globalState.userColor, globalState.isDark);
}

/**
 * Arranque: resuelve el tema y lo aplica antes del primer pintado.
 */
export function initTheme() {
    globalState.isDark = resolveInitialTheme();
    applyTheme();
}

/**
 * Cambio de tema iniciado por el usuario: persiste la preferencia y aplica.
 * @param {boolean} isDark
 */
export function setTheme(isDark) {
    globalState.isDark = isDark;
    safeStorage.setItem(THEME_KEY, isDark ? 'dark' : 'light');
    applyTheme();
}