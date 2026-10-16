<script lang="ts">
  import type { useDictation } from './state.svelte';
  import Icon from '../../shared/ui/Icon.svelte';
  let { state }: { state: ReturnType<typeof useDictation> } = $props();
  let settings = $derived(state.settings.value);
  let missing = $derived(
    settings?.microphoneId &&
      !settings.microphones.some((item) => item.id === settings?.microphoneId),
  );
  const id = 'dictation-microphone';
  async function select(control: HTMLSelectElement) {
    await state.set('microphoneId', control.value || null);
    control.value = state.settings.value?.microphoneId ?? '';
  }
</script>

{#if settings}<div class="setting-row">
    <div class="description">
      <div class="label-actions">
        <label for={id}>Микрофон</label><button
          class="quiet icon-button"
          aria-label="Обновить список микрофонов"
          title="Обновить список микрофонов"
          disabled={state.busy}
          onclick={() => state.settings.refresh()}
          ><Icon name="ph_arrows_clockwise" size={16} /></button
        >
      </div>
      <small id="microphone-help"
        >{missing
          ? 'Выбранное устройство недоступно; используется системный вход'
          : settings.microphoneId
            ? 'Изменение применяется со следующей диктовки'
            : 'Используется вход, выбранный в системных настройках'}</small
      >
    </div>
    <select
      class="setting-control"
      {id}
      aria-describedby="microphone-help"
      value={settings.microphoneId ?? ''}
      disabled={state.busy}
      onchange={(event) => select(event.currentTarget)}
    >
      <option value="">Как в системе</option>
      {#each settings.microphones as device (device.id)}<option value={device.id}
          >{device.name}</option
        >{/each}
      {#if missing}<option value={settings.microphoneId ?? ''}
          >Недоступен — используется системный</option
        >{/if}
    </select>
  </div>{/if}

<style>
  .setting-row {
    padding: 10px 0;
  }
  .description {
    flex: 1;
    min-width: 0;
  }
  .label-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .label-actions button {
    width: 24px;
    height: 24px;
    min-height: 24px;
  }
</style>
