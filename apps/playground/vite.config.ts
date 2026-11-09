/// <reference types="node" />
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { defineConfig, type Plugin } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const FEEL_WASM = fileURLToPath(new URL('../../dist/feel.wasm', import.meta.url));

/** Fail early with a useful message when the wasm core has not been built yet. */
function requireFeelWasm(): Plugin {
  return {
    name: 'require-feel-wasm',
    buildStart() {
      if (!existsSync(FEEL_WASM)) {
        this.error(`${FEEL_WASM} is missing. Build it first: \`npm run build:wasm\` (after \`npm run bootstrap\`).`);
      }
    },
  };
}

// `base: './'` keeps the build relocatable (GitHub Pages sub-path, any static file server).
export default defineConfig({
  base: './',
  plugins: [requireFeelWasm(), svelte()],
  // The runtime creates its worker with `new URL('./worker.js', import.meta.url)`; pre-bundling
  // would break that, so keep it out of the dep optimizer (it is a linked workspace package anyway).
  optimizeDeps: { exclude: ['@rheon/feel-runtime'] },
  // The runtime's worker is an ES module (it imports the WASI shim).
  worker: { format: 'es' },
  // CodeMirror + lang-feel (with its built-in docs for completion) is ~500 kB minified.
  // feel.wasm (~3.6 MB) is emitted as a separate hashed asset, never inlined.
  build: { target: 'es2022', sourcemap: true, chunkSizeWarningLimit: 700, assetsInlineLimit: 4096 },
});
