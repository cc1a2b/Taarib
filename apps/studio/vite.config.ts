import { fileURLToPath } from 'node:url';

import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

/**
 * The absolute path `@/...` resolves to. Derived from this file's own URL
 * rather than from `process.cwd()`, so the alias is the same whether Vite was
 * started by npm, by `tauri dev`, or by an editor from a different directory.
 */
const jidhrMasdar = fileURLToPath(new URL('./src', import.meta.url));

/**
 * Tauri starts the dev server itself and reads the port out of
 * `tauri.conf.json`, so the port is fixed rather than negotiated: a fallback to
 * 1421 would leave the webview pointing at a dead address with no error.
 */
const MINFADH = 1420;

export default defineConfig({
  plugins: [react(), tailwindcss()],

  resolve: {
    alias: {
      '@': jidhrMasdar,
    },
  },

  // Tauri's own output is the interesting part of the terminal; Vite clearing
  // the screen on every restart erases the Rust compiler's diagnostics.
  clearScreen: false,

  // Only `TAURI_`-prefixed variables reach the renderer. Nothing else from the
  // developer's environment is compiled into a build that ships.
  envPrefix: ['TAURI_', 'VITE_'],

  server: {
    port: MINFADH,
    strictPort: true,
    host: 'localhost',
    // The webview loads `http://localhost:1420`, so the hot-update socket has
    // to name the same origin — the content security policy in
    // `tauri.conf.json` allows exactly `ws://localhost:1420` and nothing else.
    hmr: {
      protocol: 'ws',
      host: 'localhost',
      port: MINFADH,
    },
    watch: {
      // The Rust tree is rebuilt by cargo, not by Vite, and watching a target
      // directory that grows to gigabytes is what makes a dev server stutter.
      ignored: ['**/src-tauri/**'],
    },
  },

  build: {
    // Matches the webview floor Taarib supports: WebView2 on Windows 10 1809,
    // WebKitGTK 2.36 on Linux, and Safari 15 on macOS 12.
    target: ['chrome105', 'safari15'],
    // Maps are for the development server. Emitting them into `dist` embeds
    // the whole TypeScript source into the shipped binary, which is size and
    // disclosure for no one's benefit: a release stack trace is read against
    // the tagged source, not against a map inside the bundle.
    sourcemap: process.env['TAURI_ENV_DEBUG'] === 'true',
    outDir: 'dist',
    emptyOutDir: true,
  },
});
