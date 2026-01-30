import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { nodePolyfills } from 'vite-plugin-node-polyfills'

export default defineConfig({
  root: 'src',
  plugins: [
    react(),
    nodePolyfills({
      // Include node modules that we might need
      include: ['crypto'],
      globals: {
        Buffer: true,
        global: true,
        process: true,
      },
    })
  ],
  server: {
    port: 3002,
    proxy: {
      '/v1/api': {
        target: 'http://localhost:3000',
        changeOrigin: true,
      }
    }
  },
  build: {
    // outDir: '../dist',
    emptyOutDir: true,
    commonjsOptions: {
      include: [/node_modules/, /architecture/, /cdk-arch/],
      transformMixedEsModules: true,
    },
  },
  optimizeDeps: {
    include: ['@arinoto/cdk-arch', 'architecture', 'constructs'],
    esbuildOptions: {
      mainFields: ['module', 'main'],
    }
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: './test/setup.ts',
  }
})