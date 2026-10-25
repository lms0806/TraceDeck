import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Cargo writes and locks DLLs under this directory during Tauri dev builds.
    // They are backend artifacts, never Vite source files.
    watch: { ignored: ["**/src-tauri/target/**"] }
  },
  envPrefix: ["VITE_", "TAURI_"]
});
