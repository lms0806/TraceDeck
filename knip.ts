import type { KnipConfig } from "knip";

const config: KnipConfig = {
  // Tailwind is loaded from the CSS entry point, which static JS/TS analysis
  // does not treat as a package import.
  ignoreDependencies: ["tailwindcss"],
};

export default config;
