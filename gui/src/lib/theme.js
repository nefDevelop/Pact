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
    root.style.setProperty('--md-background', `hsl(${h}, ${isDark ? 10 : 12}%, ${isDark ? 6 : 98}%)`);
    root.style.setProperty('--md-on-background', `hsl(${h}, ${isDark ? 10 : 12}%, ${isDark ? 90 : 10}%)`);
    
    root.style.setProperty('--md-surface', `hsl(${h}, ${isDark ? 8 : 10}%, ${isDark ? 10 : 100}%)`);
    root.style.setProperty('--md-on-surface', `hsl(${h}, ${isDark ? 10 : 12}%, ${isDark ? 90 : 10}%)`);
    
    root.style.setProperty('--md-surface-variant', `hsl(${h}, ${isDark ? 12 : 14}%, ${isDark ? 24 : 92}%)`);
    root.style.setProperty('--md-on-surface-variant', `hsl(${h}, ${isDark ? 10 : 12}%, ${isDark ? 80 : 30}%)`);

    // Outline
    root.style.setProperty('--md-outline', `hsl(${h}, ${isDark ? 10 : 12}%, 50%)`);
    root.style.setProperty('--md-outline-variant', `hsl(${h}, ${isDark ? 10 : 12}%, ${isDark ? 40 : 80}%)`);

    // Error
    const errorH = 0;
    root.style.setProperty('--md-error', `hsl(${errorH}, 100%, ${isDark ? 80 : 40}%)`);
    root.style.setProperty('--md-on-error', `hsl(${errorH}, 100%, ${isDark ? 20 : 100}%)`);
    root.style.setProperty('--md-error-container', `hsl(${errorH}, 100%, ${isDark ? 30 : 90}%)`);
    root.style.setProperty('--md-on-error-container', `hsl(${errorH}, 100%, ${isDark ? 90 : 10}%)`);

};
