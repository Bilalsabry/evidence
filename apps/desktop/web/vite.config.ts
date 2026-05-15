import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri 2 dev URL must match `tauri.conf.json > build.devUrl`.
const TAURI_DEV_PORT = 1420;

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: TAURI_DEV_PORT,
    strictPort: true,
  },
  // Avoid bundling Tauri's IPC API — it's injected at runtime.
  build: {
    target: "es2020",
    minify: process.env.NODE_ENV !== "development" ? "esbuild" : false,
    sourcemap: process.env.NODE_ENV === "development",
  },
});
