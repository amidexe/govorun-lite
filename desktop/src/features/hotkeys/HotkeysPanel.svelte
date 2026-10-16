<script lang="ts">
  import { useDesktopApi } from '../../shared/state/desktop-context';
  import { onMount } from 'svelte';
  import { hotkeySnapshot, type HotkeySnapshot } from './model';
  import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';
  import Icon from '../../shared/ui/Icon.svelte';
  const api = useDesktopApi();
  let { primaryOnly = false }: { primaryOnly?: boolean } = $props();
  const settings = useNativeSnapshot('hotkeys.snapshot', hotkeySnapshot, 'hotkeys.changed');
  const rows = [
    { id: 'dictation', title: 'Диктовка' },
    { id: 'plain', title: 'Диктовка без стилизации' },
    { id: 'cancel', title: 'Отмена записи' },
  ] as const;
  let busy = $state(false),
    error = $state(''),
    disposed = false;
  let pending: Promise<HotkeySnapshot> | null = null;
  const cancel = (snapshot: HotkeySnapshot | null) => {
    if (snapshot?.captureID)
      void api
        .invoke('hotkeys.action', { action: 'cancel', captureID: snapshot.captureID })
        .catch(() => {});
  };
  const cancelCapture = () => {
    cancel(settings.value);
    void pending?.then(cancel).catch(() => {});
  };
  onMount(() => {
    window.addEventListener('blur', cancelCapture);
    const unsubscribe = api.subscribe((event) => {
      if (event.event === 'dictation.changed' || event.event === 'runtime.snapshot')
        void settings.refresh();
    });
    return () => {
      disposed = true;
      unsubscribe();
      window.removeEventListener('blur', cancelCapture);
      cancelCapture();
    };
  });
  async function act(action: 'start' | 'reset', target: 'dictation' | 'plain' | 'cancel') {
    if (busy) return;
    busy = true;
    error = '';
    const request = api.invoke('hotkeys.action', { action, target }).then(hotkeySnapshot);
    pending = request;
    try {
      const result = await request;
      if (disposed) cancel(result);
      else await settings.refresh();
    } catch {
      if (!disposed) {
        error = 'Не удалось изменить сочетание. Попробуйте ещё раз.';
        await settings.refresh();
      }
    } finally {
      if (pending === request) pending = null;
      busy = false;
    }
  }
</script>

<section class="surface">
  <h2>Горячие клавиши</h2>
  {#if settings.value}
    {#each rows.filter((row) => !primaryOnly || row.id === 'dictation') as row}
      <div class="setting-row">
        <span>{row.title}</span>
        <div class="controls">
          <button
            class="quiet icon-button"
            aria-label={row.id === 'plain'
              ? 'Убрать сочетание: Диктовка без стилизации'
              : `По умолчанию: ${row.title}`}
            disabled={busy || settings.failed || !!settings.value.captureID}
            onclick={() => act('reset', row.id)}
            ><Icon name="ph_arrow_counter_clockwise" size={16} /></button
          >
          <button
            class="setting-control"
            class:capturing={settings.value.target === row.id}
            aria-label={`Изменить сочетание: ${row.title}`}
            disabled={busy || settings.failed || !settings.value.canCapture}
            onclick={() => (settings.value?.captureID ? cancelCapture() : act('start', row.id))}
          >
            {#if settings.value.target === row.id}
              {#if settings.value.preview.length}{#each settings.value.preview as token}<kbd
                    >{token}</kbd
                  >{/each}{:else}Нажмите клавишу…{/if}
            {:else if settings.value[row.id].length}{#each settings.value[row.id] as token}<kbd
                  >{token}</kbd
                >{/each}
            {:else}Назначить{/if}
          </button>
        </div>
      </div>
    {/each}
    {#if !primaryOnly}<small
        >Диктовка использует выбранный стиль и правила приложений. Дополнительное сочетание
        пропускает стилизацию только для одной записи.</small
      >{/if}
    {#if settings.value.captureID}<div class="capture-help">
        <p>Нажмите сочетание и отпустите клавиши. Esc — отмена.</p>
        <button onclick={cancelCapture}>Отмена</button>
      </div>{/if}
    {#if settings.value.message}<p class="error" role="alert">{settings.value.message}</p>{/if}
    {#if !settings.value.canCapture}<small
        >Назначение доступно с разрешением специальных возможностей, когда диктовка завершена.</small
      >{/if}
  {:else if settings.failed}<div class="notice" role="alert">
      <p>Не удалось загрузить горячие клавиши.</p>
      <button onclick={() => settings.refresh()}>Повторить</button>
    </div>
  {:else}<p role="status">Загружаем сочетания…</p>{/if}
  <small
    >{settings.value?.inputMode === 'hold'
      ? 'Удерживайте сочетание во время речи и отпустите для обработки.'
      : 'Удерживайте клавишу, чтобы говорить. Короткое нажатие включает запись без удержания.'}</small
  >
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
  .setting-row + .setting-row {
    border-top: 1px solid var(--stroke);
  }
  .controls,
  .capture-help {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .controls {
    flex-shrink: 0;
  }
  .controls button.capturing {
    border-color: var(--accent);
    color: var(--accent-text);
  }
  .capture-help {
    justify-content: space-between;
    margin: 8px 0;
  }
  kbd {
    font: inherit;
    font-size: 12px;
  }
</style>
