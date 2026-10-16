<script lang="ts">
  import { useDesktopApi } from '../../shared/state/desktop-context';
  import Modal from '../../shared/ui/Modal.svelte';
  import { recordingDiagnosticState } from './model';
  import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';
  const api = useDesktopApi();
  let { details = false }: { details?: boolean } = $props();
  const support = useNativeSnapshot(
    'recordingDiagnostics.snapshot',
    recordingDiagnosticState,
    'recording_diagnostics.changed',
  );
  let confirming = $state(false),
    busy = $state(false),
    error = $state(false),
    checked = $state(false);
  const active = $derived(support.value?.enabled === true);
  $effect(() => {
    checked = active;
  });
  function cancel() {
    confirming = false;
    checked = active;
  }
  $effect(() => {
    if (!active) return;
    const timer = setInterval(() => void support.refresh(), 2000);
    return () => clearInterval(timer);
  });
  async function act(action: 'start' | 'stop' | 'open') {
    busy = true;
    error = false;
    try {
      support.accept(
        await api.invoke(
          'recordingDiagnostics.action',
          action === 'start' ? { action, confirmed: true } : { action },
        ),
      );
      confirming = false;
    } catch {
      error = true;
    } finally {
      busy = false;
      checked = active;
    }
  }
</script>

{#if details || active}
  <section class="surface support" aria-label="Проверка начала записи">
    <h2>Проверка начала записи</h2>
    <label class="setting-row"
      ><span
        >Сохранять записи для диагностики<small
          >Три последние записи остаются на этом компьютере</small
        ></span
      >
      <input
        type="checkbox"
        role="switch"
        bind:checked
        disabled={busy || support.loading || support.failed}
        onchange={(event) => {
          if (event.currentTarget.checked) confirming = true;
          else void act('stop');
        }}
      />
    </label>
    {#if active}<p role="status">
        <strong>Сохранение звука включено.</strong> Новая запись заменяет самую старую. Снимите галочку
        после проверки.
      </p>{/if}
    {#if support.value?.hasFiles}<button disabled={busy} onclick={() => act('open')}
        >Открыть папку с записями</button
      >{/if}
    {#if details}<p class="secondary">
        Временный инструмент для поиска пропавшего начала фразы. Сохраняет первые 30 секунд звука
        микрофона, звук для модели и время событий. Файлы никуда не отправляются. Их можно
        прослушать, удалить или самостоятельно передать разработчику. При выходе из Говоруна
        сохранение выключается.
      </p>{/if}
    {#if error}<p role="alert">
        Не удалось выполнить действие. Завершите диктовку или тест скорости и повторите.
      </p>{/if}
    {#if support.failed}<p role="alert">
        Не удалось проверить состояние. <button onclick={support.refresh}>Повторить</button>
      </p>{/if}
  </section>
{/if}
{#if confirming}
  <Modal
    title="Сохранять звук для диагностики?"
    onclose={() => {
      if (!busy) cancel();
    }}
  >
    <p>
      Пока галочка включена, на диске остаются три последние записи. Новая заменяет самую старую.
      Сохраняются первые 30 секунд каждой попытки, звук для распознавателя и время событий.
    </p>
    <p>
      Проверьте фразы «Привет», «Как дела?», «Илья, привет». Не диктуйте личные сведения во время
      проверки. Автоматической отправки нет. Снятие галочки прекращает сохранение, но оставляет уже
      записанные файлы.
    </p>
    {#if error}<p role="alert">Завершите текущую диктовку или тест скорости и повторите.</p>{/if}
    {#snippet actions()}<button disabled={busy} onclick={cancel}>Отмена</button><button
        disabled={busy}
        onclick={() => act('start')}>Разрешить сохранение</button
      >{/snippet}
  </Modal>
{/if}

<style>
  .support {
    margin-bottom: 14px;
  }
  h2 {
    margin: 0 0 10px;
  }
  p {
    line-height: 1.5;
  }
  .setting-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  small {
    display: block;
    margin-top: 4px;
    color: var(--secondary);
  }
  button {
    margin: 4px 6px 4px 0;
  }
  .secondary {
    color: var(--secondary);
  }
</style>
