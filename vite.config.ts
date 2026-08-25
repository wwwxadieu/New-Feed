import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri chạy dev server ở cổng cố định và không được xoá màn hình log của Rust.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    watch: { ignored: ["**/src-tauri/**"] },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    // WebView2 trên Windows luôn là Chromium hiện đại nên có thể nhắm target cao.
    target: "chrome110",
    minify: "esbuild",
    sourcemap: false,
    chunkSizeWarningLimit: 700,
  },
});
