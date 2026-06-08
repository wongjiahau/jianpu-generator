import path from 'node:path'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      'jianpu-wasm': path.resolve(
        __dirname,
        '../crates/jianpu-wasm/pkg/jianpu_wasm.js',
      ),
    },
  },
  worker: {
    format: 'es',
  },
  server: {
    fs: {
      allow: ['..'],
    },
  },
})
