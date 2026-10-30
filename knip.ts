import { defineConfig } from "knip/config";

export default defineConfig({
  entry: ["src/main.tsx", "vite.config.ts"],
  project: ["src/**/*.{ts,tsx,css}", "vite.config.ts"],
  include: [
    "dependencies",
    "unlisted",
    "binaries",
    "unresolved",
    "exports",
    "types",
    "duplicates",
    "files",
    "cycles",
  ],
  treatConfigHintsAsErrors: true,
  treatTagHintsAsErrors: true,
  rules: {
    cycles: "error",
  },
  // These packages are loaded by Vite's CSS/module pipeline or initialized by
  // Tauri's runtime bridge, which Knip does not fully attribute to the React
  // component import graph.
  ignoreDependencies: [
    "tailwindcss",
    "@xterm/xterm",
    "@xterm/addon-fit",
    "@tauri-apps/plugin-store",
  ],
});
