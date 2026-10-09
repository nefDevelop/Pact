/**
 * Acción de Svelte que confina el foco dentro de un modal y lo devuelve al
 * elemento previo al cerrarse. Mejora la accesibilidad por teclado.
 *
 * Uso: `<div use:focusTrap role="dialog" aria-modal="true">...</div>`
 */
export function focusTrap(node) {
  const selector =
    'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

  const previouslyFocused = typeof document !== 'undefined' ? document.activeElement : null;

  function getFocusable() {
    return Array.from(node.querySelectorAll(selector)).filter(
      (el) => el.hasAttribute('aria-hidden') === false && el.getClientRects().length > 0
    );
  }

  function onKeydown(event) {
    if (event.key !== 'Tab') return;
    const focusable = getFocusable();
    if (focusable.length === 0) return;

    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }

  node.addEventListener('keydown', onKeydown);
  queueMicrotask(() => {
    const focusable = getFocusable();
    (focusable[0] || node).focus();
  });

  return {
    destroy() {
      node.removeEventListener('keydown', onKeydown);
      if (previouslyFocused && typeof previouslyFocused.focus === 'function') {
        previouslyFocused.focus();
      }
    }
  };
}
