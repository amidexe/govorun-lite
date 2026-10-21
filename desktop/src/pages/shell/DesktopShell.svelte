<script lang="ts">
  import { useDesktopApi } from '../../shared/state/desktop-context';
  import { provideDesktopNavigation } from '../../shared/state/desktop-navigation';
  import DiagnosticsPage from '../../features/diagnostics/DiagnosticsPage.svelte';
  import RecordingDiagnosticSupport from '../../features/recording-diagnostics/RecordingDiagnosticSupport.svelte';
  import AboutPage from '../../features/about/AboutPage.svelte';
  import SelfHostPage from '../../features/self-host/SelfHostPage.svelte';
  import Bird from '../../shared/ui/Bird.svelte';
  import Icon from '../../shared/ui/Icon.svelte';
  import DictionaryPage from '../../features/dictionary/DictionaryPage.svelte';
  import AccountPage from '../account/AccountPage.svelte';
  import DictationPage from '../../features/dictation/DictationPage.svelte';
  import StylesPanel from '../styles/StylesPanel.svelte';
  import ThemeChooser from '../../features/appearance/ThemeChooser.svelte';
  import { onMount } from 'svelte';
  import { initialNavigation, navigateDesktop } from './navigation';
  import { balancedScrollInsets } from '../../scrollbar';
  import type { Snippet } from 'svelte';
  const api = useDesktopApi();
  let {
    platform,
    failure = false,
    reconnecting = false,
    displayName = 'Говорун',
    recordingDiagnostics = false,
    recordingBusy = false,
    onreconnect,
    children,
  }: {
    platform: 'macos' | 'windows';
    failure?: boolean;
    reconnecting?: boolean;
    displayName?: string;
    recordingDiagnostics?: boolean;
    recordingBusy?: boolean;
    onreconnect: () => Promise<void>;
    children: Snippet;
  } = $props();
  const requestedRoute = new URLSearchParams(location.search).get('route') || 'home';
  const initial = initialNavigation(requestedRoute);
  let selected = $state(initial.selected);
  let lastSettings = initial.lastSettings;
  // Remaining native settings join this same navigation as their service commands are extracted.
  const pages = $derived([
    {
      id: 'general',
      title: 'Диктовка',
      subtitle: 'Микрофон, распознавание и разрешения',
      icon: 'ph_microphone',
    },
    {
      id: 'account',
      title: 'Аккаунт',
      subtitle: 'Вход, устройства и синхронизация',
      icon: 'ph_user_circle',
    },
    {
      id: 'styles',
      title: 'Улучшение текста',
      subtitle: 'Стиль и оформление текста',
      icon: 'ph_sparkle',
    },
    {
      id: 'llm',
      title: 'Свой сервер',
      subtitle: 'Прямое подключение к вашему API',
      icon: 'ph_hard_drives',
    },
    {
      id: 'dictionary',
      title: 'Словарь',
      subtitle: 'Замены после распознавания',
      icon: 'ph_book_open_text',
    },
    {
      id: 'diagnostics',
      title: 'Диагностика',
      subtitle: 'Состояние приложения и последние события',
      icon: 'ph_bug',
    },
    {
      id: 'about',
      title: 'О программе',
      subtitle: 'Версия, автор и открытые компоненты',
      icon: 'ph_info',
    },
  ]);
  let home = $derived(['home', 'stats'].includes(selected));
  let layoutError = $state(false);
  $effect(() => {
    let current = true;
    void api
      .invoke('window.layout', { home })
      .then(() => {
        if (current) layoutError = false;
      })
      .catch(() => {
        if (current) layoutError = true;
      });
    return () => {
      current = false;
    };
  });
  let page = $derived(
    pages.find((item) => item.id === (selected === 'subscription' ? 'account' : selected)) ??
      pages[0],
  );
  function navigate(route: string) {
    const next = navigateDesktop({ selected, lastSettings }, route);
    if (next.selected === selected && next.lastSettings === lastSettings) return;
    selected = next.selected;
    lastSettings = next.lastSettings;
    const url = new URL(location.href);
    url.searchParams.set('route', selected);
    history.replaceState(null, '', url);
  }
  provideDesktopNavigation(navigate);
  onMount(() => {
    const unsubscribe = api.subscribe((event) => {
      if (
        event.event === 'navigation' &&
        event.data &&
        typeof event.data === 'object' &&
        'route' in event.data
      ) {
        const route = event.data.route;
        if (typeof route === 'string') navigate(route);
      }
    });
    const resized = () => {
      layoutError = false;
    };
    window.addEventListener('resize', resized);
    return () => {
      unsubscribe();
      window.removeEventListener('resize', resized);
    };
  });
</script>

{#if home}
  <main class="home-layout" data-page="home">
    <header class="home-heading">
      <div class="home-brand">
        <Bird size={24} />
        <h1>{displayName}</h1>
      </div>
      <button
        class="quiet settings-button"
        aria-label="Настройки"
        title="Настройки"
        onclick={() => navigate('settings')}><Icon name="ph_gear" size={18} /></button
      >
    </header>
    <div class="home-cards page-enter" use:balancedScrollInsets>
      {#if layoutError}<p class="error" role="alert">Не удалось изменить размер окна.</p>{/if}
      {#if recordingDiagnostics}<RecordingDiagnosticSupport />{/if}
      {@render children()}
    </div>
  </main>
{:else}
  <div class="settings-layout">
    <aside>
      <button class="back quiet" aria-label="Главная" onclick={() => navigate('home')}
        ><Icon name="ph_caret_right" /><span>На главную</span></button
      >
      <nav aria-label="Настройки Говоруна">
        {#each pages as item}<button
            class:selected={page.id === item.id}
            aria-current={page.id === item.id ? 'page' : undefined}
            onclick={() => navigate(item.id)}><Icon name={item.icon} />{item.title}</button
          >{/each}
      </nav>
      <div class="appearance"><span>Оформление</span><ThemeChooser /></div>
    </aside>
    <main class="settings-content" data-page={page.id}>
      <div class="page-heading">
        <h1>{page.title}</h1>
        <p>{page.subtitle}</p>
      </div>
      {#if recordingDiagnostics}<RecordingDiagnosticSupport
          details={page.id === 'diagnostics'}
        />{/if}
      {#if layoutError}<p class="error" role="alert">Не удалось изменить размер окна.</p>{/if}
      {#if failure}<div class="notice" role="alert">
          Диктовка временно недоступна.
          <button onclick={onreconnect} disabled={reconnecting}
            >{reconnecting ? 'Подключаемся…' : 'Попробовать ещё раз'}</button
          >
        </div>{/if}
      {#key page.id}<div class="page-enter">
          {#if page.id === 'general'}<DictationPage
              {platform}
              {recordingBusy}
            />{:else if page.id === 'account'}<AccountPage
              subscriptionOpen={selected === 'subscription'}
            />{:else if page.id === 'styles'}<StylesPanel
            />{:else if page.id === 'llm'}<SelfHostPage
            />{:else if page.id === 'diagnostics'}<DiagnosticsPage
              {platform}
              {recordingBusy}
            />{:else if page.id === 'about'}<AboutPage
              {platform}
            />{:else if page.id === 'dictionary'}<DictionaryPage />{/if}
        </div>{/key}
    </main>
  </div>
{/if}

<style>
  .settings-layout {
    display: grid;
    grid-template-columns: 184px minmax(0, 1fr);
    height: 100vh;
  }
  aside {
    overflow-y: auto;
    background: var(--sidebar);
    padding: 16px 12px;
    border-right: 1px solid var(--stroke);
    display: flex;
    flex-direction: column;
  }
  .home-layout {
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .home-heading,
  .home-brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .home-heading {
    justify-content: space-between;
    height: 52px;
    flex-shrink: 0;
    padding: 0 16px;
  }
  .home-heading h1 {
    margin: 0;
    font-size: 17px;
  }
  .settings-button {
    width: 30px;
    height: 30px;
    min-height: 30px;
    padding: 0;
  }
  /* Reserve the gutter without showing a scrollbar when the cards fit. */
  .home-cards {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-gutter: stable;
    padding-block: 14px;
    padding-left: max(14px, var(--scrollbar-width, 0px));
    padding-right: max(0px, calc(14px - var(--scrollbar-width, 0px)));
  }
  :global(html[data-platform='macos']) .home-cards {
    overflow-y: scroll;
  }
  .home-cards :global(.surface) {
    margin-bottom: 10px;
    padding: 14px;
    border: 0;
    box-shadow: none;
  }
  .home-cards :global(.home-statistics),
  .home-cards :global(.last-dictation),
  .home-cards :global(.styles-panel.compact) {
    padding: 0;
  }
  .home-cards :global(.expander) {
    height: 52px;
    min-height: 52px;
    padding: 0 14px;
  }
  .home-cards :global(.surface:last-child) {
    margin-bottom: 0;
  }
  .home-cards :global(h2) {
    font-size: 13px;
  }
  .home-cards :global(.expander strong) {
    font-weight: 500;
  }
  .back {
    justify-content: flex-start;
    gap: 8px;
    padding: 10px;
    margin-bottom: 20px;
  }
  .back :global(.icon) {
    transform: rotate(180deg);
  }
  nav {
    display: grid;
    gap: 5px;
  }
  nav button {
    justify-content: flex-start;
    border: 0;
    background: transparent;
    border-radius: 10px;
    padding: 12px;
    gap: 10px;
    text-align: left;
  }
  nav button.selected {
    background: var(--surface);
    box-shadow: var(--control-shadow);
    color: var(--accent-text);
    font-weight: 600;
  }
  .appearance {
    margin-top: auto;
    padding: 18px 6px 0;
  }
  .appearance > span {
    display: block;
    margin: 0 0 8px 2px;
    color: var(--secondary);
    font-size: 11px;
  }
  .settings-content {
    overflow: auto;
    padding: 24px;
  }
  .page-heading {
    margin-bottom: 22px;
  }
</style>
