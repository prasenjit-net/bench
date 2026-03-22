import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  // In dev, proxy API calls to the Rust server (port supplied via env or 7878)
  server: {
    proxy: {
      '/api': `http://localhost:${process.env.BENCH_PORT ?? 7878}`,
    },
  },
})
