import path from 'node:path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  // relative asset paths: `strata site` の出力(ui/dist を graph.json と並べて
  // 合成)は file:// でもサーバでも自己完結で開けるようにする(D50 の「静的=配布形態」)。
  base: './',
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
})
