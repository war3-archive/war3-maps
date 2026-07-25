import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { fileURLToPath, URL } from "node:url";

const rootDir = fileURLToPath(new URL(".", import.meta.url));
const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const wasmDist = fileURLToPath(new URL("../dist", import.meta.url));

// Project Pages: https://wesleyel.github.io/war3parser/
// Local / preview: relative assets also work with "./"
const base = process.env.GITHUB_PAGES === "1" ? "/war3parser/" : "./";

export default defineConfig({
  root: rootDir,
  base,
  plugins: [react()],
  resolve: {
    alias: {
      "@wesleyel/war3parser": wasmDist,
    },
  },
  server: {
    fs: {
      allow: [repoRoot],
    },
  },
  optimizeDeps: {
    exclude: ["@wesleyel/war3parser"],
  },
  build: {
    outDir: "dist-site",
    emptyOutDir: true,
    sourcemap: true,
  },
});
