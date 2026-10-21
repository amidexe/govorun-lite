<script lang="ts">
  import { useDesktopApi } from './shared/state/desktop-context';
  import { onMount } from 'svelte';
  import { runtimeSnapshot, type RuntimeSnapshot } from './features/dictation/runtime';
  import OnboardingGate from './features/setup/OnboardingGate.svelte';
  import DesktopShell from './pages/shell/DesktopShell.svelte';
  import HomeStatistics from './features/statistics/HomeStatistics.svelte';
  import UpdateCard from './features/updates/UpdateCard.svelte';
  import HotkeyHint from './features/hotkeys/HotkeyHint.svelte';
  import LastDictationCard from './features/clipboard/LastDictationCard.svelte';
  import StylesPanel from './pages/styles/StylesPanel.svelte';
  import SetupDictation from './features/setup/SetupDictation.svelte';
  import { dictationReady } from './features/setup/model';
  import { homeStatus } from './pages/home/readiness';
  import Icon from './shared/ui/Icon.svelte';
  import { observeScrollActivity } from './scrollbar';
  const api = useDesktopApi();
  let snapshot = $state<RuntimeSnapshot | null>(null);
  let failure = $state(false);
  let reconnecting = $state(false);
  let repairing = $state(false);
  let preparing = $state(false),
    prepareError = $state('');
  let status = $derived(homeStatus(snapshot));
  let allReady = $derived(
    !failure &&
      dictationReady(snapshot) &&
      ['Всё готово', 'Диктовка готова'].includes(status.title),
  );
  $effect(() => {
    if (snapshot) document.documentElement.dataset.platform = snapshot.platform;
  });
  $effect(() => {
    if (!failure && dictationReady(snapshot)) repairing = false;
  });
  let snapshotGeneration = 0;
  async function prepare() {
    preparing = true;
    prepareError = '';
    try {
      await api.invoke('dictation.action', { action: 'prepare' });
    } catch {
      prepareError = 'Не удалось начать подготовку. Попробуйте ещё раз.';
    } finally {
      preparing = false;
    }
  }
  async function reconnect() {
    const generation = ++snapshotGeneration;
    reconnecting = true;
    try {
      const value = runtimeSnapshot(await api.invoke('runtime.reconnect'));
      if (generation === snapshotGeneration) {
        snapshot = value;
        failure = false;
      }
    } catch {
      if (generation === snapshotGeneration) failure = true;
    } finally {
      reconnecting = false;
    }
  }
  onMount(() => {
    const stopScrollActivity = observeScrollActivity();
    const refresh = () => {
      const generation = ++snapshotGeneration;
      void api
        .invoke('runtime.snapshot')
        .then((value) => {
          if (generation === snapshotGeneration) {
            snapshot = runtimeSnapshot(value);
            failure = false;
          }
        })
        .catch(() => {
          if (generation === snapshotGeneration) failure = true;
        });
    };
    refresh();
    window.addEventListener('focus', refresh);
    // Device presence can change without a permission change or window focus.
    const readinessTimer = window.setInterval(() => {
      if (document.visibilityState === 'visible' && snapshot?.phase === 'idle') refresh();
    }, 2000);
    const unsubscribe = api.subscribe((event) => {
      if (event.event === 'runtime.snapshot') {
        snapshotGeneration++;
        try {
          snapshot = runtimeSnapshot(event.data);
          failure = false;
        } catch {
          failure = true;
        }
      }
      if (event.event === 'runtime.unavailable') {
        snapshotGeneration++;
        failure = true;
      }
    });
    return () => {
      snapshotGeneration++;
      unsubscribe();
      stopScrollActivity();
      window.removeEventListener('focus', refresh);
      window.clearInterval(readinessTimer);
    };
  });
</script>

<OnboardingGate runtime={snapshot} {failure} {reconnecting} onreconnect={reconnect}>
  <DesktopShell
    platform={snapshot?.platform ?? 'macos'}
    {failure}
    {reconnecting}
    displayName={snapshot?.displayName}
    recordingDiagnostics={snapshot?.capabilities.includes('recordingDiagnostics.action') ?? false}
    recordingBusy={snapshot?.phase === 'starting' ||
      snapshot?.phase === 'recording' ||
      snapshot?.phase === 'processing'}
    onreconnect={reconnect}
  >
    <section class="surface readiness-card" class:ready={allReady} aria-live="polite">
      <div class="status-line">
        {#if allReady}<span class="success"><Icon name="ph_check_circle_fill" size={16} /></span
          >{/if}
        <h2>{failure ? 'Диктовка недоступна' : status.title}</h2>
        {#if failure || status.detail}<p>
            {failure ? 'Попробуйте запустить её ещё раз.' : status.detail}
          </p>{/if}
      </div>
      {#if failure}<button onclick={reconnect} disabled={reconnecting}
          >{reconnecting ? 'Подключаемся…' : 'Попробовать ещё раз'}</button
        >{/if}
      {#if !failure && status.retry}<button onclick={prepare} disabled={preparing}
          >{preparing ? 'Подготавливаем…' : 'Повторить подготовку'}</button
        >{/if}
      {#if !failure && snapshot?.phase === 'idle' && snapshot.readiness && !dictationReady(snapshot)}<button
          class="quiet"
          onclick={() => {
            repairing = !repairing;
          }}>{repairing ? 'Свернуть настройку' : 'Микрофон и клавиши'}</button
        >{/if}
      {#if prepareError}<p class="error" role="alert">{prepareError}</p>{/if}
    </section>
    {#if repairing && !failure}<SetupDictation runtime={snapshot} />{/if}
    {#if allReady && snapshot?.capabilities.includes('statistics.snapshot')}<HotkeyHint />{/if}
    {#if snapshot?.capabilities.includes('lastDictation.snapshot')}<LastDictationCard
        phase={snapshot.phase}
      />{/if}
    {#if snapshot?.capabilities.includes('styles.snapshot')}<StylesPanel
        compact
        processingStatus={snapshot.styleStatus ?? 'idle'}
      />{/if}
    {#if snapshot?.capabilities.includes('statistics.snapshot')}<HomeStatistics />{/if}
    <UpdateCard compact />
  </DesktopShell>
</OnboardingGate>

<style>
  .status-line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 10px;
  }
  .status-line h2 {
    margin: 0;
    font-size: 13px;
    font-weight: 500;
  }
  .status-line p {
    flex-basis: 100%;
    font-size: 11px;
  }
  .ready .status-line {
    flex-wrap: nowrap;
    min-height: 24px;
  }
  .ready .status-line p {
    flex: 1;
    text-align: right;
    color: var(--green);
  }
  .status-line .success {
    display: flex;
  }
  .readiness-card > button {
    margin-top: 10px;
  }
</style>
