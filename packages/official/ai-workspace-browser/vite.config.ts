import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  packages: [react()],
  root: '.',
  base: './',
  build: {
    outDir: 'dist/renderer',
    emptyOutDir: false
  },
  server: {
    port: 5179,
    strictPort: true
  }
});
