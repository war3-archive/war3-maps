import { defineConfig } from "astro/config";

export default defineConfig({
  site: "https://war3-archive.github.io",
  base: "/war3-maps/",
  output: "static",
  vite: {
    // wasm-pack's `--target web` output resolves its .wasm through
    // `import.meta.url`, which Vite's dependency pre-bundling rewrites to a
    // path that does not exist. Leaving the package unbundled keeps that URL
    // pointing at the real file.
    optimizeDeps: { exclude: ["@wesleyel/war3parser"] },
  },
});
