import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";

export default defineConfig({
  root: fileURLToPath(new URL(".", import.meta.url)),
  base: "./",
  resolve: {
    dedupe: ["react", "react-dom"],
  },
  build: {
    outDir: fileURLToPath(new URL("../../out/functor_web", import.meta.url)),
    emptyOutDir: true,
  },
});
