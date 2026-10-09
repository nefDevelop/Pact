import { defineConfig, mergeConfig } from 'vitest/config';
import viteConfig from './vite.config.js';

// La build SSR de Svelte no expone `mount`. Forzamos la condición `browser`
// para que los tests de componentes usen la build cliente.
export default mergeConfig(
  viteConfig,
  defineConfig({
    resolve: {
      conditions: ['browser', 'module', 'import', 'default'],
    },
    ssr: {
      resolve: {
        conditions: ['browser', 'module', 'import', 'default'],
      },
    },
  })
);