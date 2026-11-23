import { defineConfig } from "knip/config";

export default defineConfig({
  entry: ["src/main.tsx", "vite.config.ts"],
  project: ["src/**/*.{ts,tsx,css}", "vite.config.ts"],
  include: [
    "dependencies",
    "unlisted",
    "binaries",
    "unresolved",
    "catalog",
    "catalogReferences",
    "exports",
    "nsExports",
    "types",
    "nsTypes",
    "enumMembers",
    "namespaceMembers",
    "duplicates",
    "files",
    "cycles",
  ],
  includeEntryExports: true,
  treatConfigHintsAsErrors: true,
  treatTagHintsAsErrors: true,
  rules: {
    cycles: "error",
    exports: "error",
  },
  ignoreExportsUsedInFile: {
    type: true,
    interface: true,
  },
  // Knip does not attribute these runtime/CSS-backed imports to the final
  // Tauri entry graph, although workspace.ts and TerminalDock.tsx use them.
  ignoreDependencies: [
    "@tauri-apps/plugin-store",
    "@xterm/addon-fit",
    "@xterm/xterm",
  ],
});
