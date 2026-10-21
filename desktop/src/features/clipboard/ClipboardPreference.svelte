<script lang="ts">
  import { useDesktopApi } from '../../shared/state/desktop-context';
  import { clipboardSnapshot, type ClipboardChange } from './model';
  import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';
  const api = useDesktopApi();
  let { recordingBusy = false }: { recordingBusy?: boolean } = $props();
  const output = useNativeSnapshot('clipboard.snapshot', clipboardSnapshot, 'clipboard.changed');
  let busy = $state(false),
    error = $state(false);
  async function save(change: ClipboardChange, restore: () => void) {
    if (busy || recordingBusy || output.failed || !output.value) return;
    busy = true;
    error = false;
    try {
      output.accept(await api.invoke('clipboard.action', change));
    } catch {
      error = true;
    } finally {
      busy = false;
      restore();
    }
  }
  function change(control: HTMLSelectElement) {
    if (control.value !== 'copy' && control.value !== 'preserve') return;
    void save({ clipboard: control.value }, () => {
      if (output.value) control.value = output.value.clipboard;
    });
  }
  function remember(control: HTMLInputElement) {
    void save({ rememberLast: control.checked }, () => {
      if (output.value) control.checked = output.value.rememberLast;
    });
  }
</script>

{#if output.failed}<p class="notice" role="alert">
    Не удалось загрузить настройки. <button onclick={output.refresh}>Повторить</button>
  </p>{/if}
{#if output.value}
  <label class="setting-row" for="clipboard-handling">
    <span>После вставки текста<small>Что останется в буфере обмена после диктовки</small></span>
    <select
      class="setting-control"
      id="clipboard-handling"
      value={output.value.clipboard}
      disabled={busy || recordingBusy || output.failed}
      onchange={(event) => change(event.currentTarget)}
    >
      <option value="preserve">Сохранить прежний буфер</option>
      <option value="copy">Оставить последний текст</option>
    </select>
  </label>
  <label class="setting-row"
    ><span
      >Помнить последнюю диктовку<small
        >Только в памяти, до следующей диктовки или выхода. Скопировать её можно на главном экране и
        из меню значка Говоруна.</small
      ></span
    ><input
      type="checkbox"
      role="switch"
      id="remember-last-dictation"
      checked={output.value.rememberLast}
      disabled={busy || recordingBusy || output.failed}
      onchange={(event) => remember(event.currentTarget)}
    /></label
  >
{:else if output.loading}<p role="status">Загружаем настройки…</p>{/if}
{#if error}<p class="error" role="alert">
    Не удалось сохранить настройку. Прежнее значение сохранено.
  </p>{/if}

<style>
  .setting-row {
    border-top: 1px solid var(--stroke);
    padding: 10px 0;
  }
</style>
