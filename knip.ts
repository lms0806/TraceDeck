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
  // Tailwind is loaded from the CSS entry point, which static JS/TS analysis
  // does not treat as a package import.
  ignoreDependencies: ["tailwindcss"],
});
