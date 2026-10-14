import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    // `.svelte.ts` modules hold runes and must resolve before plain `.ts`.
    extensions: ['.svelte.ts', '.svelte.js', '.mjs', '.js', '.ts', '.json'],
    alias: {
      $lib: fileURLToPath(new URL('./src/lib', import.meta.url)),
      $domain: fileURLToPath(new URL('./src/domain', import.meta.url)),
      $services: fileURLToPath(new URL('./src/services', import.meta.url)),
      $stores: fileURLToPath(new URL('./src/stores', import.meta.url)),
      $components: fileURLToPath(new URL('./src/components', import.meta.url)),
      $i18n: fileURLToPath(new URL('./src/i18n/index.svelte.ts', import.meta.url)),
    },
  },
  server: {
    port: 1420,
    strictPort: true,
  },
});
