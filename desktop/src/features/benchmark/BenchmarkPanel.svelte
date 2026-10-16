<script lang="ts">
  import { useDesktopApi } from '../../shared/state/desktop-context';
  import { benchmarkSnapshot, benchmarkSpeed, backendLabel } from './model';
  import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';
  const api = useDesktopApi();
  let {
    platform,
    recordingBusy = false,
  }: { platform: 'macos' | 'windows'; recordingBusy?: boolean } = $props();
  const benchmark = useNativeSnapshot('benchmark.snapshot', benchmarkSnapshot, 'benchmark.changed');
  let busy = $state(false),
    error = $state(false),
    copied = $state(false);
  const running = $derived(benchmark.value?.state === 'running');
  const devices = $derived(benchmark.value?.devices ?? []);
  const active = $derived(devices.find((item) => item.id === benchmark.value?.activeDeviceId));
  const number = new Intl.NumberFormat('ru', { maximumFractionDigits: 1 });
  const stageNames: Record<string, string> = {
    devices: 'Определяем доступные устройства…',
    fixtures: 'Проверяем примеры…',
    model: 'Загружаем модель…',
    cold: '1 из 3 · Первый запуск…',
    warm: '2 из 3 · Повторный запуск…',
    long: '3 из 3 · Длинная запись…',
  };
  const failureNames: Record<string, string> = {
    samples_unavailable: 'Встроенные примеры недоступны.',
    model_missing: 'Модель распознавания не найдена.',
    runtime_incompatible: 'Не удалось загрузить движок распознавания.',
    initialization_failed: 'Не удалось подготовить модель.',
    recognition_failed: 'Ошибка при распознавании примера.',
  };
  async function act(action: 'start' | 'cancel' | 'copy') {
    if (busy) return;
    busy = true;
    error = false;
    copied = false;
    try {
      benchmark.accept(await api.invoke('benchmark.action', { action }));
      copied = action === 'copy';
    } catch {
      error = true;
    } finally {
      busy = false;
    }
  }
</script>

<section class="surface benchmark" aria-label="Проверка распознавания">
  <header>
    <h2>Проверка распознавания</h2>
    <p>Процессор и все доступные видеокарты — по очереди, на одинаковых примерах. Без микрофона.</p>
  </header>
  <div class="actions">
    {#if running}<button disabled={busy} onclick={() => act('cancel')}>Отменить тест</button>
    {:else}<button
        class="primary"
        disabled={busy || recordingBusy || benchmark.loading || benchmark.failed}
        onclick={() => act('start')}>Запустить тест</button
      >{/if}
    <button
      disabled={busy || running || !benchmark.value || benchmark.failed}
      onclick={() => act('copy')}>Скопировать отчёт</button
    >
    {#if copied}<span role="status">Отчёт скопирован</span>{/if}
  </div>
  {#if running}<p class="status" role="status">
      {active ? `${active.name} · ` : ''}{stageNames[benchmark.value!.stage]} Диктовка временно приостановлена.
    </p>{/if}
  {#if recordingBusy && !running}<p class="status" role="status">
      Завершите диктовку, чтобы начать проверку.
    </p>{/if}
  {#if benchmark.failed}<div class="status actions" role="alert">
      <p>Не удалось подключить тест.</p>
      <button onclick={benchmark.refresh}>Попробовать снова</button>
    </div>{/if}
  {#if error}<p class="status error" role="alert">
      Не удалось выполнить действие. Дождитесь готовности распознавания и попробуйте снова.
    </p>{/if}
  {#if benchmark.value?.state === 'failed'}<p class="status error" role="alert">
      {devices.length
        ? 'Не все устройства прошли проверку. Успешные результаты сохранены.'
        : benchmark.value.failureCode
          ? failureNames[benchmark.value.failureCode]
          : 'Тест не завершён.'} Подробности — в отчёте.
    </p>{/if}
  {#if benchmark.value?.state === 'cancelled'}<p class="status" role="status">
      Тест отменён. Завершённые результаты сохранены, диктовка снова доступна.
    </p>{/if}
  {#if devices.length}
    <ul class="results" aria-label="Результаты по устройствам" aria-live="polite">
      {#each devices as device (device.id)}
        <li class="device-result">
          <div class="device-description">
            <strong>{device.name}</strong><span>{backendLabel(device.backend)}</span>
          </div>
          <div class="device-outcome">
            {#if device.result}<strong class="speed"
                >×{number.format(benchmarkSpeed(device.result))}</strong
              ><span>быстрее длительности записи</span>
            {:else if device.state === 'failed'}<span class="error"
                >{failureNames[device.failureCode!]}</span
              >
            {:else if device.state === 'running'}<span>{stageNames[device.stage]}</span>
            {:else if device.state === 'cancelled'}<span>Отменено</span>
            {:else}<span>{running ? 'Ожидает проверки' : 'Не проверено'}</span>{/if}
          </div>
        </li>
      {/each}
    </ul>
    {#if !running && devices.every((item) => item.backend === 'cpu')}<p class="status">
        Видеокарта с поддержкой {platform === 'macos' ? 'Metal' : 'Vulkan'} недоступна для проверки.
      </p>{/if}
  {/if}
  <details>
    <summary>Технический отчёт</summary>
    <pre>{benchmark.value?.report ?? 'Загружаем сведения…'}</pre>
  </details>
</section>

<style>
  .benchmark {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-width: 0;
  }
  h2 {
    margin: 0 0 6px;
  }
  header p {
    color: var(--secondary);
  }
  .actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
  }
  .actions button {
    min-height: 36px;
  }
  .status {
    font-size: 12px;
  }
  .results {
    display: flex;
    flex-direction: column;
    gap: 0;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .device-result {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 16px;
    padding: 12px 0;
    border-top: 1px solid var(--stroke);
  }
  .device-description,
  .device-outcome {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .device-description {
    flex: 1 1 170px;
  }
  .device-description strong {
    font-size: 13px;
  }
  .device-description span,
  .device-outcome span {
    color: var(--secondary);
    font-size: 12px;
  }
  .device-outcome .error {
    color: var(--red);
  }
  .speed {
    font-size: 26px;
  }
  details {
    border-top: 1px solid var(--stroke);
    padding-top: 12px;
  }
  summary {
    cursor: pointer;
    color: var(--secondary);
    font-size: 12px;
  }
  pre {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    font-size: 11px;
    line-height: 1.6;
    margin: 12px 0 0;
  }
  .error {
    color: var(--red);
  }
</style>
