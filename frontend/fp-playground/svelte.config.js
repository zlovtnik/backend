import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/vite-plugin-svelte').SvelteConfig} */
const config = {
  preprocess: vitePreprocess(),
  compilerOptions: {
    compatibility: {
      componentApi: 4
    }
  }
};

export default config;
