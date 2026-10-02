import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  base: './',
  plugins: [
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
