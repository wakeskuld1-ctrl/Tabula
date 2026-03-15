import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import path from 'path'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    // ### 变更记录
    // - 2026-03-11 23:06: 原因=前台已彻底切到 GlideGrid 路线; 目的=移除 Wasm 专用构建插件，避免残留配置干扰。
    // - 2026-03-12 21:50: 原因=移除 apiShim，前端通过 GridAPI 直连后端 /api/execute; 目的=消除中间件层。
    react()
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
