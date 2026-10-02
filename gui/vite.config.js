import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import wasm from 'vite-plugin-wasm'

// https://vite.dev/config/
export default defineConfig({
  base: './',
  plugins: [
    // `pact-core` se compila a WASM (make build-wasm) para que el navegador
    // pueda derivar las claves con el mismo Argon2id que el escritorio, en vez
    // de depender del IPC de Tauri.
    wasm(),
    svelte(),
    tailwindcss(),
  ],
  server: {
    port: 8088,
    strictPort: true,
    proxy: {
      '/status': 'http://localhost:8002',
      '/auth': 'http://localhost:8002',
      '/vault': 'http://localhost:8002',
      '/sync': 'http://localhost:8002',
      '/system': 'http://localhost:8002',
    }
  },
  test: {
    environment: 'jsdom',
    globals: true,
  }
})
