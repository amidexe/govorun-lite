<script lang="ts">
  import { useDictation } from './state.svelte';
  import PermissionSetup from './PermissionSetup.svelte';
  import MicrophonePicker from './MicrophonePicker.svelte';
  import HotkeysPanel from '../hotkeys/HotkeysPanel.svelte';
  import ClipboardPreference from '../clipboard/ClipboardPreference.svelte';
  import { gpuLabel } from '../benchmark/model';
  import type { DictationFields } from '../../ipc/actions';
  import { choice } from '../../shared/forms/choice';
  let {
    platform,
    recordingBusy = false,
  }: { platform: 'macos' | 'windows'; recordingBusy?: boolean } = $props();
  const state = useDictation();
  let settings = $derived(state.settings.value);
  // Keep existing custom limits available alongside the common durations.
  let durations = $derived(
    [
      ...new Set([
        ...Array.from({ length: 37 }, (_, index) => index * 5),
        settings?.maxMinutes ?? 0,
      ]),
    ].sort((a, b) => a - b),
  );
  async function change<Field extends keyof DictationFields>(
    control: HTMLInputElement | HTMLSelectElement,
    field: Field,
    value: DictationFields[Field],
  ) {
    await state.set(field, value);
    const current = state.settings.value?.[field];
    if (control instanceof HTMLInputElement && control.type === 'checkbox')
      control.checked = current === true;
    else control.value = current === null || current === undefined ? '' : String(current);
  }
</script>

{#if state.settings.failed}<div class="notice" role="alert">
    <p>Не удалось загрузить настройки диктовки.</p>
    <button onclick={() => state.settings.refresh()}>Попробовать ещё раз</button>
  </div>{/if}
{#if settings}
  <HotkeysPanel />
  <section class="surface">
    <h2>Распознавание</h2>
    <label class="setting-row"
      ><span
        >Максимальная длительность<small
          >{settings.durationMode === 'unlimited'
            ? 'Со своим сервером длительность не ограничивается'
            : settings.durationMode === 'styles'
              ? 'Со стилями запись целиком завершится через 5 минут'
              : 'После лимита запись завершится и отправится на распознавание'}</small
        ></span
      >
      <select
        class="setting-control"
        aria-label="Максимальная длительность"
        value={settings.maxMinutes}
        disabled={state.busy || settings.durationMode === 'unlimited'}
        onchange={(event) =>
          change(event.currentTarget, 'maxMinutes', Number(event.currentTarget.value))}
      >
        {#each durations as minutes}<option value={minutes}
            >{minutes === 0 ? 'Без ограничения' : `${minutes} мин`}</option
          >{/each}
      </select>
    </label>
    <label class="setting-row"
      ><span>Ускорение<small>Если GPU недоступен, используется CPU</small></span>
      <select
        class="setting-control"
        aria-label="Ускорение"
        value={settings.acceleration}
        disabled={state.busy}
        onchange={(event) =>
          change(
            event.currentTarget,
            'acceleration',
            choice(event.currentTarget.value, ['automatic', 'cpu', 'gpu'] as const),
          )}
        ><option value="automatic">{gpuLabel(platform)}</option><option value="cpu"
          >CPU · Процессор</option
        >{#if settings.inputChannels !== undefined}<option value="gpu"
            >GPU · Vulkan (принудительно)</option
          >{/if}</select
      >
    </label>
  </section>
  <section class="surface">
    <h2>Результат диктовки</h2>
    <label class="setting-row"
      ><span
        >Помечать голосовой текст<small
          >Показывает собеседнику, что сообщение продиктовано · от {settings.markerMinimum} символов</small
        ></span
      >
      <select
        class="setting-control"
        aria-label="Помечать голосовой текст"
        value={settings.voiceMarker}
        disabled={state.busy}
        onchange={(event) =>
          change(
            event.currentTarget,
            'voiceMarker',
            choice(event.currentTarget.value, ['off', 'microphone', 'text'] as const),
          )}
        ><option value="off">Не помечать</option><option value="microphone"
          >Краткий — 🔊 в начале</option
        ><option value="text">Подробный — строка в конце</option></select
      >
    </label>
    <ClipboardPreference {recordingBusy} />
  </section>
  <section class="surface">
    <h2>Звук</h2>
    <MicrophonePicker {state} />
    {#if settings.alwaysOnMicrophone !== undefined}
      <label class="setting-row"
        ><span
          >Держать микрофон включённым<small
            >Сокращает задержку начала записи. Пока работает Говорун, системный индикатор микрофона
            остаётся включённым. Звук вне диктовки отбрасывается.</small
          ></span
        ><input
          type="checkbox"
          role="switch"
          aria-label="Держать микрофон включённым"
          checked={settings.alwaysOnMicrophone}
          disabled={state.busy || recordingBusy}
          onchange={(event) =>
            change(event.currentTarget, 'alwaysOnMicrophone', event.currentTarget.checked)}
        /></label
      >
    {/if}
    <label class="setting-row"
      ><span>Звуки записи<small>Сигнал в начале и конце диктовки</small></span><input
        type="checkbox"
        role="switch"
        checked={settings.playSounds}
        disabled={state.busy}
        onchange={(event) => change(event.currentTarget, 'playSounds', event.currentTarget.checked)}
      /></label
    >
    <label class="setting-row"
      ><span
        >Приглушать звук<small>Уменьшать громкость других приложений во время записи</small></span
      ><input
        type="checkbox"
        role="switch"
        checked={settings.muteAudio}
        disabled={state.busy}
        onchange={(event) => change(event.currentTarget, 'muteAudio', event.currentTarget.checked)}
      /></label
    >
  </section>
  {#if settings.inputChannels !== undefined}
    <details class="surface advanced-dictation">
      <summary>Дополнительные параметры</summary>
      <label class="setting-row"
        ><span
          >Запись с удержанием<small
            >Удерживайте сочетание во время речи и отпустите для обработки</small
          ></span
        ><input
          type="checkbox"
          role="switch"
          checked={settings.pushToTalk}
          disabled={state.busy}
          onchange={(event) =>
            change(event.currentTarget, 'pushToTalk', event.currentTarget.checked)}
        /></label
      >
      <label class="setting-row"
        ><span
          >Держать распознавание готовым<small
            >Выключите, чтобы освобождать память после 15 минут без диктовки</small
          ></span
        ><input
          type="checkbox"
          role="switch"
          checked={settings.keepModelLoaded}
          disabled={state.busy}
          onchange={(event) =>
            change(event.currentTarget, 'keepModelLoaded', event.currentTarget.checked)}
        /></label
      >
      <label class="setting-row"
        ><span>Канал микрофона</span><select
          class="setting-control"
          value={settings.inputChannel ?? ''}
          disabled={state.busy}
          onchange={(event) =>
            change(
              event.currentTarget,
              'inputChannel',
              event.currentTarget.value === '' ? null : Number(event.currentTarget.value),
            )}
          ><option value="">Все каналы</option
          >{#each Array.from({ length: Math.max(settings.inputChannels, (settings.inputChannel ?? -1) + 1) }, (_, index) => index) as channel}<option
              value={channel}>Канал {channel + 1}</option
            >{/each}</select
        ></label
      >
    </details>
  {/if}
  <section class="surface">
    <h2>Система</h2>
    <label class="setting-row"
      ><span>Запускать при входе в систему<small>Говорун будет готов сразу после входа</small></span
      >
      <input
        type="checkbox"
        role="switch"
        aria-label="Запускать при входе в систему"
        checked={settings.launchAtLogin === 'enabled'}
        disabled={state.busy || settings.launchAtLogin === 'unavailable'}
        onchange={async (event) => {
          const control = event.currentTarget;
          await state.set('launchAtLogin', control.checked);
          control.checked = state.settings.value?.launchAtLogin === 'enabled';
        }}
      />
    </label>
    {#if settings.launchAtLogin === 'approvalRequired'}<div class="notice">
        <p>Разрешите автозапуск в системных настройках.</p>
        <button disabled={state.busy} onclick={() => state.act({ action: 'loginItems' })}
          >Открыть настройки</button
        >
      </div>
    {:else if settings.launchAtLogin === 'unavailable'}<p>
        Автозапуск станет доступен после установки приложения.
      </p>{/if}
  </section>
  <section class="surface">
    <h2>Разрешения</h2>
    <PermissionSetup
      permissions={settings.permissions}
      busy={state.busy}
      onopen={(permission) => state.act({ action: 'permission', permission })}
      onrefresh={() => state.settings.refresh()}
    />
  </section>
{:else if state.settings.loading}<section class="surface">
    <p role="status">Загружаем настройки диктовки…</p>
  </section>{/if}
{#if state.error}<p class="error" role="alert">{state.error}</p>{/if}

<style>
  .setting-row + .setting-row {
    border-top: 1px solid var(--stroke);
  }
  .setting-row {
    padding: 10px 0;
  }
  .setting-row > span:first-child {
    flex: 1;
    min-width: 0;
  }
</style>
