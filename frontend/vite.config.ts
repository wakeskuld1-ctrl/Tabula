import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'
import path from 'path'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    react(),
    wasm(),
    topLevelAwait()
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      'react': path.resolve(__dirname, 'node_modules/react'),
      'react-dom': path.resolve(__dirname, 'node_modules/react-dom'),
      '@univerjs/core': path.resolve(__dirname, 'node_modules/@univerjs/core'),
      '@univerjs/design': path.resolve(__dirname, 'node_modules/@univerjs/design'),
      '@univerjs/engine-render': path.resolve(__dirname, 'node_modules/@univerjs/engine-render'),
      '@univerjs/engine-formula': path.resolve(__dirname, 'node_modules/@univerjs/engine-formula'),
      '@univerjs/ui': path.resolve(__dirname, 'node_modules/@univerjs/ui'),
      '@univerjs/sheets': path.resolve(__dirname, 'node_modules/@univerjs/sheets'),
      '@univerjs/sheets-ui': path.resolve(__dirname, 'node_modules/@univerjs/sheets-ui'),
      '@univerjs/docs': path.resolve(__dirname, 'node_modules/@univerjs/docs'),
      '@univerjs/docs-ui': path.resolve(__dirname, 'node_modules/@univerjs/docs-ui'),
    },
  },
  server: {
    host: '0.0.0.0',
    port: 5174,
    fs: {
      allow: ['..']
    },
    proxy: {
      '/api': {
        target: 'http://localhost:3000',
        changeOrigin: true,
      }
    }
  }
})
