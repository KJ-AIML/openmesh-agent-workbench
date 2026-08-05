import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  // Only scan the main index.html for dependency pre-bundling.
  // This prevents Vite from trying to resolve deps referenced by
  // unrelated HTML files elsewhere in the repo (e.g. skills/, examples/).
  optimizeDeps: {
    entries: ["index.html"],
    // React island for freeform Board (@excalidraw/excalidraw).
    include: ["react", "react-dom", "react/jsx-runtime", "@excalidraw/excalidraw"],
  },
  // Restrict file serving to project root + node_modules only.
  server: {
    port: 3000,
    host: true,
    strictPort: true,
    fs: {
      allow: [__dirname, "node_modules"],
    },
  },
  preview: {
    port: 3000,
    host: true,
    strictPort: true,
  },
});
