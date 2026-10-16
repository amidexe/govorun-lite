import { mount } from 'svelte';
import App from './App.svelte';
import { installDesktopApi } from './desktop-api';
import './theme.css';
import { createAppearance, appearanceKey } from './features/appearance/state.svelte';
import { createUpdates, updatesKey } from './features/updates/state.svelte';
import { desktopApiKey } from './shared/state/desktop-context';

void installDesktopApi()
  .then(async (api) => {
    const appearance = createAppearance(api);
    await appearance.initialize();
    mount(App, {
      target: document.getElementById('app')!,
      context: new Map<unknown, unknown>([
        [desktopApiKey, api],
        [appearanceKey, appearance],
        [updatesKey, createUpdates(api)],
      ]),
    });
  })
  .catch(() => {
    const root = document.getElementById('app')!;
    const message = document.createElement('p');
    message.textContent = 'Не удалось открыть интерфейс Говоруна.';
    const retry = document.createElement('button');
    retry.textContent = 'Попробовать ещё раз';
    retry.onclick = () => location.reload();
    root.replaceChildren(message, retry);
  });
