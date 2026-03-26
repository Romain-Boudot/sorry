import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      "/api": "http://localhost:3000",
      "/ws": { target: "ws://localhost:3000", ws: true },
    },
  },
  build: {
    outDir: "dist",
  },
});
