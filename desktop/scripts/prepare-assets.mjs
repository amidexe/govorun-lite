import { mkdir, readFile, readdir, copyFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { resolve, join } from 'node:path';

const root = fileURLToPath(new URL('../../', import.meta.url));
const output = resolve(root, 'desktop/public/generated');
await mkdir(output, { recursive: true });
await copyFile(resolve(root, 'desktop/resources/Geist.ttf'), join(output, 'Geist.ttf'));
const available = new Set();
const assets = resolve(root, 'macos/Govorun/Assets.xcassets');
for (const directory of await readdir(assets)) {
  if (!directory.startsWith('ph_') || !directory.endsWith('.imageset')) continue;
  const svg = (await readdir(join(assets, directory))).find(name => name.endsWith('.svg'));
  if (svg) {
    available.add(directory.replace('.imageset', ''));
    await copyFile(join(assets, directory, svg), join(output, directory.replace('.imageset', '.svg')));
  }
}

async function verifyReferencedIcons(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) await verifyReferencedIcons(path);
    else if (/\.(svelte|ts)$/.test(entry.name)) {
      const source = await readFile(path, 'utf8');
      for (const [icon] of source.matchAll(/\bph_[a-z0-9_]+\b/g)) {
        if (!available.has(icon)) throw new Error(`Missing bundled icon: ${icon}`);
      }
    }
  }
}
await verifyReferencedIcons(resolve(root, 'desktop/src'));
