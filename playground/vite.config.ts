import { defineConfig } from "vite";
import { fileURLToPath, URL } from "node:url";

const rootDir = fileURLToPath(new URL(".", import.meta.url));
const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const wasmDist = fileURLToPath(new URL("../dist", import.meta.url));

export default defineConfig({
  root: rootDir,
  base: "./",
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
  },
});
