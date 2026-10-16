<script lang="ts">
  import Icon from '../../shared/ui/Icon.svelte';
  import { useUpdates } from './state.svelte';
  import { updateMessage, downloadProgress } from './model';
  import { fade } from 'svelte/transition';
  import { motionDuration } from '../../motion';
  let { compact = false }: { compact?: boolean } = $props();
  const updates = useUpdates();
  let release = $derived(updates.snapshot?.release);
  let visible = $derived(!compact || (release && !updates.snapshot?.dismissed));
  let status = $derived(updates.snapshot?.status);
  function update() { void updates.act(status === 'ready' ? 'install' : 'download'); }
  let active = $derived(status === 'downloading' || status === 'installing');
</script>

{#if visible}
  <section
    class="surface update-card"
    class:compact
    aria-label="Обновления"
    transition:fade={{ duration: motionDuration() }}
  >
    <div class="heading">
      <Icon name="ph_arrows_clockwise" />
      <h2>{release ? `Доступна версия ${release.version}` : 'Обновления'}</h2>
    </div>
    {#if !compact}<p>
        {updateMessage(updates.busy ? 'checking' : (updates.snapshot?.status ?? 'idle'))}
      </p>{/if}
    {#if !compact && release && updates.snapshot?.status === 'unavailable'}<p role="status">
        {updateMessage('unavailable')}
      </p>{/if}
    {#if active || (status === 'ready' && !compact)}<div class="transfer" role="status">
        <p>{updateMessage(status!)}</p>
        {#if status === 'downloading' && updates.snapshot?.transfer}<progress
            max={updates.snapshot.transfer.total}
            value={updates.snapshot.transfer.downloaded}
            aria-label="Загрузка обновления"
          ></progress>
          <p>{downloadProgress(updates.snapshot.transfer)}</p>{/if}
      </div>{/if}
    {#if release && !active && status !== 'ready' && (!compact || !updates.error)}<p
        class="restart-note"
      >
        {compact
          ? 'Приложение перезапустится'
          : 'После загрузки и проверки приложение перезапустится. Текущая диктовка должна завершиться.'}
      </p>{/if}
    <div class="actions">
      {#if status === 'downloading'}<button onclick={() => updates.act('cancel')}
          >Отменить загрузку</button
        >
      {:else if release}<button
          class="primary"
          disabled={updates.opening || updates.busy || status === 'installing'}
          onclick={update}
          >{status === 'installing'
            ? 'Устанавливаем…'
            : updates.opening
              ? 'Подготавливаем…'
                : status === 'ready'
                ? 'Перезапустить и обновить'
                : 'Обновить'}</button
        >{/if}
      {#if compact}<button
          class="quiet"
          disabled={updates.opening || updates.busy || active}
          onclick={() => updates.act('dismiss')}>Позже</button
        >
      {:else}<button
          disabled={updates.busy || updates.opening || active || status === 'ready'}
          onclick={() => updates.check(true)}
          >{updates.busy ? 'Проверяем…' : 'Проверить обновления'}</button
        >{/if}
    </div>
    {#if updates.error}<p class="error" role="alert">{updates.error}</p>{/if}
    {#if !compact && updates.snapshot?.channelSelectable}
      <details class="channel">
        <summary>Дополнительные параметры</summary>
        <label for="update-channel">Канал обновлений</label>
        <select
          id="update-channel"
          value={updates.snapshot.channel}
          disabled={updates.busy || updates.opening || active || status === 'checking'}
          onchange={(event) =>
            updates.selectChannel(event.currentTarget.value as 'stable' | 'test')}
        >
          <option value="stable">Стабильный</option><option value="test">Тестовый (Beta)</option>
        </select>
        <p>
          В тестовом канале новые версии появляются раньше. Выбранный канал применяется к следующим обновлениям. Установка начнётся только по нажатию кнопки.
        </p>
      </details>
    {/if}
  </section>
{/if}

<style>
  .heading {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  h2 {
    margin: 0;
    font-size: 13px;
  }
  p {
    margin: 8px 0 0;
    color: var(--secondary);
    line-height: 1.5;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 12px;
  }
  .error {
    color: var(--red);
  }
  .compact {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: 12px;
  }
  .compact .heading {
    grid-column: 1;
  }
  .compact .actions {
    grid-column: 2;
    grid-row: 1 / span 2;
    flex-direction: column;
    gap: 4px;
    margin: 0;
  }
  .compact .error,
  .compact .transfer {
    grid-column: 1;
    grid-row: 2;
    font-size: 12px;
    line-height: 18px;
  }
  .compact .transfer p,
  .compact .error {
    margin-top: 4px;
  }
  .compact progress {
    display: block;
    height: 6px;
    margin-top: 4px;
  }
  .transfer,
  .restart-note {
    grid-column: 1 / -1;
  }
  progress {
    width: 100%;
    margin-top: 8px;
  }
  .compact .restart-note {
    font-size: 12px;
    line-height: 16px;
    margin-top: 0;
  }
  .channel {
    margin-top: 16px;
  }
  .channel summary {
    cursor: pointer;
    color: var(--secondary);
  }
  .channel label {
    display: block;
    margin: 12px 0 8px;
  }
  .channel select {
    width: 100%;
  }
</style>
