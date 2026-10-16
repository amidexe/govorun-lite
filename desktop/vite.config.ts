import { defineConfig } from 'vite';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { svelte } from '@sveltejs/vite-plugin-svelte';

const config = JSON.parse(
  readFileSync(new URL('./src-tauri/tauri.conf.json', import.meta.url), 'utf8'),
);
const date = new Date().toISOString();
const buildID =
  process.env.GOVORUN_BUILD_ID ??
  date.slice(0, 10).replaceAll('-', '') + '.' + date.slice(11, 16).replace(':', '') + 'Z';
if (!/^\d{8}\.\d{4}Z$/.test(buildID)) throw new Error('Invalid GOVORUN_BUILD_ID');

export default defineConfig({
  define: { __DESKTOP_BUILD__: JSON.stringify({ version: config.version, buildID }) },
  plugins: [svelte()],
  base: './',
  server: { host: '127.0.0.1', port: 1420, strictPort: true },
  build: {
    target: 'es2022',
    sourcemap: false,
    rollupOptions: {
      input: {
        main: fileURLToPath(new URL('./index.html', import.meta.url)),
        recording: fileURLToPath(new URL('./recording.html', import.meta.url)),
      },
    },
  },
});
