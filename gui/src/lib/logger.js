/**
 * Logging acotado al modo desarrollo.
 *
 * En producción no queremos volcar por consola cada petición ni los mensajes de
 * diagnóstico: solo hacen ruido. `console.error`/`warn` de errores reales siguen
 * saliendo; aquí únicamente se silencia `log`/`debug`.
 */

const isDev = import.meta.env?.DEV === true;

/** Log de diagnóstico. Silencioso en builds de producción. */
export const devLog = (...args) => {
    if (isDev) console.log(...args);
};

/** Log de depuración. Silencioso en builds de producción. */
export const devDebug = (...args) => {
    if (isDev) console.debug(...args);
};

/** Log de advertencia. Se muestra siempre (son casos que requieren atención). */
export const devWarn = (...args) => console.warn(...args);

/** Log de error. Se muestra siempre. */
export const devError = (...args) => console.error(...args);

export const isDevMode = isDev;