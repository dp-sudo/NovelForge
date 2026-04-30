import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes("node_modules")) {
            return;
          }
          if (
            id.includes("/react/") ||
            id.includes("/react-dom/") ||
            id.includes("/scheduler/")
          ) {
            return "react-vendor";
          }
          if (id.includes("/@tauri-apps/")) {
            return "tauri-vendor";
          }
          if (id.includes("/@radix-ui/")) {
            return "radix-vendor";
          }
          if (
            id.includes("/zustand/") ||
            id.includes("/@tanstack/react-query/")
          ) {
            return "state-vendor";
          }
          return "vendor";
        },
      },
    },
  },
  server: {
    port: 5173,
    strictPort: true
  }
});
