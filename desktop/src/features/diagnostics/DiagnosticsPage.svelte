<script lang="ts">
  import { useDesktopApi } from '../../shared/state/desktop-context';
  import BenchmarkPanel from '../benchmark/BenchmarkPanel.svelte';
  let {
    platform,
    recordingBusy = false,
  }: { platform: 'macos' | 'windows'; recordingBusy?: boolean } = $props();
  import Icon from '../../shared/ui/Icon.svelte';
  import Modal from '../../shared/ui/Modal.svelte';
  import { diagnosticsSnapshot } from './model';
  import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';
  const api = useDesktopApi();
  const journal = useNativeSnapshot(
    'diagnostics.snapshot',
    diagnosticsSnapshot,
    'diagnostics.changed',
    api,
    { nativeRequired: false },
  );
  let confirming = $state(false),
    busy = $state(false),
    error = $state(false),
    copied = $state<'journal' | 'report' | null>(null),
    saved = $state(false);
  $effect(() => {
    if (journal.failed) {
      confirming = false;
      copied = null;
    }
  });
  async function act(action: 'copy' | 'copyReport' | 'clear') {
    if (busy || journal.failed) return;
    busy = true;
    error = false;
    copied = null;
    try {
      const result = await api.invoke(
        'diagnostics.action',
        action === 'clear' ? { action, confirmed: true } : { action },
      );
      if (action === 'clear') {
        journal.accept(result);
        confirming = false;
      } else copied = action === 'copy' ? 'journal' : 'report';
    } catch {
      error = true;
    } finally {
      busy = false;
    }
  }
  const formatDate = (date: string) =>
    new Intl.DateTimeFormat('ru', {
      day: 'numeric',
      month: 'short',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    }).format(new Date(date));
  async function configureExtended(enabled: boolean) {
    if (busy || journal.failed) return;
    busy = true;
    error = false;
    copied = null;
    try {
      journal.accept(await api.invoke('diagnostics.action', { action: 'extended', enabled }));
    } catch {
      error = true;
    } finally {
      busy = false;
    }
  }
  async function saveReport() {
    if (busy || journal.failed) return;
    busy = true;
    error = false;
    saved = false;
    try {
      const report = await api.invoke('diagnostics.action', { action: 'export' });
      if (report !== null) throw new Error('invalid_report');
      saved = true;
    } catch {
      error = true;
    } finally {
      busy = false;
    }
  }
</script>

<BenchmarkPanel {platform} {recordingBusy} />
{#if journal.failed}<div class="notice" role="alert">
    Не удалось обновить журнал. <button onclick={journal.refresh}>Повторить</button>
  </div>{/if}
<section class="surface ordinary">
  <h2>События приложения</h2>
  <p class="secondary">Ошибки и события работы. Без текстов диктовок.</p>
  <div class="actions">
    <button
      disabled={busy ||
        journal.failed ||
        journal.value?.nativeAvailable === false ||
        !journal.value?.events.length}
      onclick={() => act('copy')}><Icon name="ph_copy" />Скопировать журнал</button
    >
    <button
      class="destructive quiet"
      disabled={busy ||
        journal.failed ||
        journal.value?.nativeAvailable === false ||
        !(journal.value?.events.length || journal.value?.extendedEvents?.length)}
      onclick={() => {
        confirming = true;
        error = false;
      }}><Icon name="ph_trash" />Очистить</button
    >
    {#if copied === 'journal'}<span role="status">Журнал скопирован</span>{/if}
  </div>
  {#if error && !confirming}<p class="error" role="alert">
      Не удалось выполнить действие. Попробуйте ещё раз.
    </p>{/if}
</section>
<section class="surface journal" aria-label="Последние события">
  <h2>Последние события</h2>
  {#if journal.value}
    {#each journal.value.events as event (event.id)}
      <article class:warning={event.level === 'warning'} class:error={event.level === 'error'}>
        <Icon name={event.level === 'info' ? 'ph_check_circle_fill' : 'ph_warning_circle_fill'} />
        <div class="event-content">
          {#snippet heading()}
            <div class="event-heading">
              <strong>{event.category}</strong><time datetime={event.date}
                >{formatDate(event.date)}</time
              ><span class="level"
                >{event.level === 'error'
                  ? 'Ошибка'
                  : event.level === 'warning'
                    ? 'Внимание'
                    : 'Информация'}</span
              >
            </div>
            <p>{event.summary}</p>
          {/snippet}
          {#if event.details}<details class="event-disclosure">
              <summary>
                <div class="event-label">{@render heading()}</div>
                <span class="caret"><Icon name="ph_caret_down" /></span>
                {#if event.compact}<span class="compact">{event.compact}</span>{/if}
              </summary>
              <div class="details selectable">
                {#each event.details.split('\n') as line}<p class:stage={/^[1-3]\. /.test(line)}>
                    {line.trimStart()}
                  </p>{/each}
              </div>
            </details>
          {:else}{@render heading()}{/if}
        </div>
      </article>
    {:else}<p class="secondary">Пока нет событий.</p>{/each}
  {:else if journal.loading}<p role="status">Загружаем журнал…</p>{/if}
</section>
<details class="advanced">
  <summary>Дополнительные параметры</summary>
  <section class="surface">
    <h2>Технический отчёт</h2>
    <p class="secondary">
      Версия приложения, сохранённые события и подробности сбоев. Отчёт остаётся на этом компьютере.
    </p>
    {#if journal.value?.extendedEnabled !== undefined}
      <label class="extended"
        ><input
          type="checkbox"
          checked={journal.value.extendedEnabled}
          disabled={busy || journal.failed}
          onchange={(event) => configureExtended(event.currentTarget.checked)}
        />
        Расширенная диагностика</label
      >
      <p class="secondary">
        Сохранять сведения о запуске, остановке обработчика и системных ошибках. Без текста, звука
        диктовок и личных данных. Не более 200 событий.
      </p>
    {/if}
    {#if journal.value?.nativeAvailable === false}<p class="notice" role="alert">
        Обработчик не отвечает. Сохраните отчёт: в него войдут сведения, сохранённые приложением.
      </p>{/if}
    {#if journal.value}<p class="secondary">
        Расширенных событий: {journal.value.extendedEvents?.length ?? 0}.
      </p>{/if}
    <div class="actions">
      <button
        disabled={busy || journal.failed || journal.value?.nativeAvailable === false}
        onclick={() => act('copyReport')}>Скопировать отчёт</button
      >
      {#if journal.value?.extendedEnabled !== undefined}<button
          disabled={busy || journal.failed}
          onclick={saveReport}>Сохранить</button
        >{/if}
    </div>
    {#if error && !confirming}<p class="error" role="alert">
        Не удалось выполнить действие. Попробуйте ещё раз.
      </p>{/if}
    {#if copied === 'report'}<p role="status">Отчёт скопирован</p>{/if}
    {#if saved}<p role="status">Отчёт сохранён в папке «Загрузки».</p>{/if}
  </section>
</details>
{#if confirming}
  <Modal
    title="Очистить журнал?"
    onclose={() => {
      if (!busy) confirming = false;
    }}
  >
    <p>Будут удалены сохранённые события приложения и сведения об ошибке установки.</p>
    {#if error}<p class="error" role="alert">
        Не удалось очистить журнал. Попробуйте ещё раз.
      </p>{/if}
    {#snippet actions()}<button
        disabled={busy}
        onclick={() => {
          confirming = false;
        }}>Отмена</button
      ><button class="danger" disabled={busy} onclick={() => act('clear')}
        >{busy ? 'Очищаем…' : 'Очистить'}</button
      >{/snippet}
  </Modal>
{/if}

<style>
  .surface + .surface,
  .advanced {
    margin-top: 14px;
  }
  h2 {
    margin: 0 0 10px;
  }
  .secondary {
    color: var(--secondary);
    line-height: 1.55;
  }
  .actions {
    margin-top: 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .extended {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .advanced > summary {
    cursor: pointer;
    color: var(--secondary);
    padding: 14px 0;
  }
  article {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 14px 0;
  }
  article + article {
    border-top: 1px solid var(--stroke);
  }
  .event-content {
    min-width: 0;
    flex: 1;
  }
  .event-heading {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px 12px;
  }
  time,
  .level {
    font-size: 11px;
    color: var(--secondary);
  }
  article p {
    color: var(--text);
    margin: 7px 0;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  summary {
    cursor: pointer;
    color: var(--secondary);
    padding: 4px 0;
  }
  .event-disclosure > summary {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    list-style: none;
    padding: 0;
  }
  .event-disclosure > summary::-webkit-details-marker {
    display: none;
  }
  .caret {
    align-self: center;
  }
  .event-disclosure[open] .caret {
    transform: rotate(180deg);
  }
  .compact {
    grid-column: 1;
    font-size: 12px;
    color: var(--secondary);
  }
  .event-disclosure[open] .compact {
    display: none;
  }
  .details {
    white-space: pre-wrap;
    margin-top: 8px;
  }
  .details p {
    margin: 0;
    font-size: 12px;
    color: var(--secondary);
  }
  .details p.stage {
    margin-top: 10px;
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }
</style>
