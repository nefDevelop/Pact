/**
 * M3 Theme Utilities
 */

/**
 * @param {string} hexVal
 * @returns {number[]} [h, s, l]
 */
export const hexToHsl = (hexVal) => {
    let r = parseInt(hexVal.slice(1, 3), 16) / 255;
    let g = parseInt(hexVal.slice(3, 5), 16) / 255;
    let b = parseInt(hexVal.slice(5, 7), 16) / 255;
    let max = Math.max(r, g, b), min = Math.min(r, g, b);
    let h, s, l = (max + min) / 2;
    if (max === min) h = s = 0;
    else {
        let d = max - min;
        s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
        switch (max) {
            case r: h = (g - b) / d + (g < b ? 6 : 0); break;
            case g: h = (b - r) / d + 2; break;
            case b: h = (r - g) / d + 4; break;
        }
        h /= 6;
    }
    return [h * 360, s * 100, l * 100];
};

/**
 * Applies Material You (M3) tonal palette to the document.
 * @param {string} hex Seed color
 * @param {boolean} isDark 
 */
export const applyM3Palette = (hex, isDark) => {
    if (!hex || typeof document === 'undefined') return;
    
    const hsl = hexToHsl(hex);
    if (!hsl || hsl.length < 3) return;
    const [h, s] = hsl;
    
    const root = document.documentElement;
    const targetS = isDark ? Math.min(s, 40) : s;

    const setProp = (name, l_val, s_offset = 0) => {
        const finalS = Math.max(0, targetS + s_offset);
        root.style.setProperty(`--md-${name}`, `hsl(${h}, ${finalS}%, ${l_val}%)`);
    };

    // Primary
    setProp('primary', isDark ? 80 : 40);
    setProp('on-primary', isDark ? 20 : 100);
    setProp('primary-container', isDark ? 30 : 90);
    setProp('on-primary-container', isDark ? 90 : 10);
    
    // Secondary & Tertiary (desaturated)
    const sOffset = -20;
    setProp('secondary', isDark ? 80 : 40, sOffset);
    setProp('on-secondary', isDark ? 20 : 100, sOffset);
    setProp('secondary-container', isDark ? 30 : 90, sOffset);
    setProp('on-secondary-container', isDark ? 90 : 10, sOffset);

    // Background & Surface
    // Canvas: en claro queda claramente por debajo de las superficies (91%), de
    // modo que el blanco de paneles y tarjetas recorta sobre el fondo. El tinte
    // es bajo a propósito (10%) para que el lienzo sea un gris roto, no lavanda.
    root.style.setProperty('--md-background', `hsl(${h}, ${isDark ? 12 : 10}%, ${isDark ? 9 : 91}%)`);
    root.style.setProperty('--md-on-background', `hsl(${h}, ${isDark ? 10 : 12}%, ${isDark ? 90 : 10}%)`);
    
    root.style.setProperty('--md-surface', `hsl(${h}, ${isDark ? 8 : 10}%, ${isDark ? 13 : 100}%)`);
    root.style.setProperty('--md-on-surface', `hsl(${h}, ${isDark ? 10 : 12}%, ${isDark ? 90 : 10}%)`);
    
    root.style.setProperty('--md-surface-variant', `hsl(${h}, ${isDark ? 12 : 14}%, ${isDark ? 25 : 88}%)`);
    root.style.setProperty('--md-on-surface-variant', `hsl(${h}, ${isDark ? 10 : 12}%, ${isDark ? 80 : 30}%)`);

    // Outline
    root.style.setProperty('--md-outline', `hsl(${h}, ${isDark ? 10 : 12}%, ${isDark ? 55 : 55}%)`);
    root.style.setProperty('--md-outline-variant', `hsl(${h}, ${isDark ? 10 : 12}%, ${isDark ? 32 : 76}%)`);

    // Borde semántico (estilo mixto, sutil) y superficies de elevación M3.
    // En oscuro la separación se consigue con tinte de superficie + borde,
    // porque la sombra negra es invisible sobre el fondo.
    root.style.setProperty('--md-border', `hsl(${h}, ${isDark ? 10 : 12}%, ${isDark ? 34 : 80}%)`);
    // Las superficies flotantes (modales, login, notificaciones) usan un filete
    // algo más fuerte: su borde exterior convive con la sombra y en las esquinas
    // redondeadas se lava si usa el mismo token que los separadores.
    // En claro NO se refuerza: el lienzo (91%) ya separa por tono de la tarjeta
    // (97.5%), y un filete fuerte solo hace más visible el antialiasado de la
    // curva. En oscuro sí hace falta, porque las superficies están muy juntas.
    root.style.setProperty('--md-border-window', `hsl(${h}, ${isDark ? 10 : 12}%, ${isDark ? 40 : 80}%)`);
    root.style.setProperty('--md-surface-1', `hsl(${h}, ${isDark ? 9 : 14}%, ${isDark ? 16 : 98.5}%)`);
    root.style.setProperty('--md-surface-2', `hsl(${h}, ${isDark ? 10 : 16}%, ${isDark ? 19 : 97.5}%)`);
    root.style.setProperty('--md-surface-3', `hsl(${h}, ${isDark ? 12 : 18}%, ${isDark ? 22 : 96}%)`);

    // Elevación: en oscuro, sombra suave + realce superior (inset) que hace legible
    // el nivel de altura; en claro, sombras tintadas con el hue de la semilla.
    // Escala contenida a propósito: con el lienzo ya separado por tinte, una
    // sombra fuerte se acumula y ensucia los elementos grandes.
    if (isDark) {
        root.style.setProperty('--elev-1', 'inset 0 1px 0 rgba(255, 255, 255, 0.05), 0 1px 2px rgba(0, 0, 0, 0.35)');
        root.style.setProperty('--elev-2', 'inset 0 1px 0 rgba(255, 255, 255, 0.06), 0 3px 10px rgba(0, 0, 0, 0.40)');
        root.style.setProperty('--elev-3', 'inset 0 1px 0 rgba(255, 255, 255, 0.07), 0 10px 26px rgba(0, 0, 0, 0.50)');
    } else {
        root.style.setProperty('--elev-1', `0 1px 2px hsl(${h} 30% 18% / 0.05), 0 1px 3px hsl(${h} 30% 18% / 0.04)`);
        root.style.setProperty('--elev-2', `0 3px 8px hsl(${h} 30% 18% / 0.07), 0 1px 3px hsl(${h} 30% 18% / 0.04)`);
        root.style.setProperty('--elev-3', `0 10px 26px hsl(${h} 30% 18% / 0.10), 0 3px 6px hsl(${h} 30% 18% / 0.05)`);
    }

    // Error
    const errorH = 0;
    root.style.setProperty('--md-error', `hsl(${errorH}, 100%, ${isDark ? 80 : 40}%)`);
    root.style.setProperty('--md-on-error', `hsl(${errorH}, 100%, ${isDark ? 20 : 100}%)`);
    root.style.setProperty('--md-error-container', `hsl(${errorH}, 100%, ${isDark ? 30 : 90}%)`);
    root.style.setProperty('--md-on-error-container', `hsl(${errorH}, 100%, ${isDark ? 90 : 10}%)`);

};
