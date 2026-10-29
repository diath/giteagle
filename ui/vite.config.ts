import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Tauri expects a fixed port and its own output in the terminal, so Vite
// must neither pick another port nor clear the screen.
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    chunkSizeWarningLimit: 1500,
  },
  // The highlighting worker loads grammars on demand, which needs a module
  // worker; the default classic format can't split code.
  worker: {
    format: 'es',
  },
})
