import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    target: ['es2021', 'chrome100', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    rollupOptions: {
      output: {
        // 固定 vendor chunk，提升缓存命中；同时避免过细分包导致 Rollup circular chunk 警告。
        manualChunks: {
          'vendor-react': ['react', 'react-dom', 'react-router-dom'],
          'vendor-antd': ['antd', '@ant-design/pro-components'],
          'vendor-chart': ['echarts', 'echarts-for-react', 'recharts'],
          'vendor-query': ['@tanstack/react-query'],
        },
        chunkFileNames: 'assets/[name]-[hash].js',
        entryFileNames: 'assets/[name]-[hash].js',
        assetFileNames: 'assets/[name]-[hash].[ext]',
      },
    },
    // 说明：antd/echarts 等依赖体积天然较大。已通过 manualChunks 做基础分包，并提高告警阈值避免噪音。
    chunkSizeWarningLimit: 2000,
  },
})
