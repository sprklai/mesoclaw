import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { paraglideVitePlugin } from "@inlang/paraglide-js";
import { defineConfig } from "vite";

export default defineConfig({
  server: {
    port: 18971,
    strictPort: true,
    fs: {
      allow: [".", "./messages"],
    },
  },
  plugins: [
    tailwindcss(),
    paraglideVitePlugin({
      project: "./project.inlang",
      outdir: "./src/lib/paraglide",
    }),
    sveltekit(),
  ],
  build: {
    // L15 fix: split heavy dependencies into separate lazy-loaded chunks so the
    // initial bundle stays lean. D3, Shiki, and svelte-streamdown are only needed
    // on the wiki/chat routes and should not inflate the main entry chunk.
    rollupOptions: {
      output: {
        manualChunks(id: string) {
          if (
            id.includes("/node_modules/d3") ||
            id.includes("/node_modules/d3-")
          ) {
            return "d3";
          }
          if (id.includes("/node_modules/shiki")) {
            return "shiki";
          }
          if (id.includes("/node_modules/svelte-streamdown")) {
            return "streamdown";
          }
        },
      },
    },
  },
});
