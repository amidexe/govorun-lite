<script lang="ts">
  import type { RuntimeSnapshot } from '../dictation/runtime';
  import { useDictation } from '../dictation/state.svelte';
  import PermissionSetup from '../dictation/PermissionSetup.svelte';
  import MicrophonePicker from '../dictation/MicrophonePicker.svelte';
  import HotkeysPanel from '../hotkeys/HotkeysPanel.svelte';
  import Icon from '../../shared/ui/Icon.svelte';
  let { runtime }: { runtime: RuntimeSnapshot | null } = $props();
  const state = useDictation();
  let model = $derived(runtime?.readiness?.model);
  let ready = $derived(model === 'ready' || model === 'standby');
</script>

{#if state.settings.failed}<div class="notice" role="alert">
    <p>Не удалось проверить разрешения.</p>
    <button onclick={() => state.settings.refresh()}>Проверить ещё раз</button>
  </div>{/if}
<section class="surface">
  {#if state.settings.value}<PermissionSetup
      permissions={state.settings.value.permissions}
      busy={state.busy}
      onopen={(permission) => state.act({ action: 'permission', permission })}
      onrefresh={() => state.settings.refresh()}
    />{:else}<p role="status">Проверяем разрешения…</p>{/if}
  <div class="model-row">
    <span class:success={ready}
      ><Icon name={ready ? 'ph_check_circle_fill' : 'ph_warning_circle_fill'} /></span
    >
    <div>
      <strong>Распознавание на устройстве</strong>
      <p role="status">
        {ready
          ? 'Готово. Голос не отправляется в интернет.'
          : model === 'failed'
            ? 'Не удалось подготовить распознавание. Попробуйте ещё раз.'
            : model === 'loading'
              ? 'Подготавливаем распознавание…'
              : 'Подготовьте распознавание для первой диктовки.'}
      </p>
      {#if model && ['failed', 'unprepared', 'stopped'].includes(model)}<button
          disabled={state.busy}
          onclick={() => state.act({ action: 'prepare' })}
          >{model === 'failed' ? 'Повторить подготовку' : 'Подготовить распознавание'}</button
        >{/if}
    </div>
  </div>
  {#if runtime?.readiness?.accessibility && !runtime.readiness.hotkey}<p
      class="notice"
      role="status"
    >
      Проверяем горячую клавишу. Если доступ выдан, а она не подключается, перезапустите Говорун.
    </p>{/if}
</section>
<HotkeysPanel primaryOnly />
<details><summary>Выбрать микрофон</summary><MicrophonePicker {state} /></details>
{#if state.error}<p class="error" role="alert">{state.error}</p>{/if}

<style>
  details {
    margin: 14px 0;
  }
  summary {
    cursor: pointer;
    color: var(--secondary);
  }
  .model-row {
    border-top: 1px solid var(--stroke);
    display: flex;
    gap: 10px;
    padding: 12px 0;
  }
  .model-row p {
    margin-top: 4px;
  }
  .model-row button {
    margin-top: 8px;
  }
</style>
