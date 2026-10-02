import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { initTheme } from './lib/themeMode.js'

// Resolvemos tema y paleta ANTES de montar: así el primer frame ya lleva los
// colores correctos y no hay salto de claro a oscuro.
initTheme()

const app = mount(App, {
  target: document.getElementById('app'),
})

export default app