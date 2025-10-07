import { defineConfig } from 'vite'
import viteReact from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

import { TanStackRouterVite } from '@tanstack/router-plugin/vite'
import { resolve } from 'node:path'
import tsconfigPaths from 'vite-tsconfig-paths'

// https://vitejs.dev/config/
export default defineConfig({
  base: '/admin/',
  plugins: [
    tsconfigPaths(),
    TanStackRouterVite({ autoCodeSplitting: true }),
    viteReact(),
    tailwindcss(),
  ],
  server: {
    proxy: {
      '/rpc': {
        target: 'http://localhost:3030',
        changeOrigin: true,
      },
    }
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
    },
  },
})
