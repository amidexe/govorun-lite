<script lang="ts">
  import { onDestroy } from 'svelte';
  import { useDesktopApi } from '../../shared/state/desktop-context';
  import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';
  import Modal from '../../shared/ui/Modal.svelte';
  import { useAccount } from '../account/state.svelte';
  import { statisticsSnapshot } from './model';
  import { statisticsResetController } from './state';
  const api = useDesktopApi();
  const account = useAccount();
  const statistics = useNativeSnapshot(
    'statistics.snapshot',
    statisticsSnapshot,
    'statistics.changed',
  );
  let revision = $state(0);
  const reset = statisticsResetController({
    scope: () => account.account.value?.scope ?? null,
    available: () =>
      !!statistics.value &&
      !statistics.failed &&
      !account.account.failed &&
      !account.account.value?.busy,
    reset: () => api.invoke('statistics.reset', { confirmed: true }),
    accept: (snapshot) => statistics.accept(snapshot),
    changed: () => {
      revision++;
    },
  });
  let view = $derived.by(() => {
    revision;
    return reset.view;
  });
  $effect(() => {
    revision;
    reset.synchronize();
  });
  onDestroy(reset.dispose);
</script>

<section class="surface">
  <h2>Статистика</h2>
  <p>Слова, время речи, сессии и история по дням.</p>
  <button class="danger quiet" disabled={!view.enabled} onclick={reset.request}
    >Сбросить статистику</button
  >
  {#if statistics.failed || account.account.failed}<p class="error" role="alert">
      Сброс недоступен. Обновите сведения и попробуйте снова.
      <button
        onclick={() => {
          void statistics.refresh();
          void account.account.refresh();
        }}>Повторить</button
      >
    </p>{/if}
  {#if view.failed}<p class="error" role="alert">
      Не удалось подтвердить сброс. Обновите статистику и проверьте результат.
    </p>{/if}
  {#if view.completed}<p role="status">Статистика сброшена</p>{/if}
</section>
{#if view.confirming}<Modal title="Сбросить всю статистику?" onclose={reset.cancel}>
    <p>Будут удалены слова, время речи, сессии и история по дням.</p>
    <p>Это действие нельзя отменить.</p>
    {#snippet actions()}<button class="quiet" onclick={reset.cancel}>Отмена</button>
      <button class="danger" onclick={reset.confirm}>Сбросить статистику</button>{/snippet}
  </Modal>{/if}

<style>
  h2 {
    margin: 0 0 12px;
  }
  p {
    margin: 12px 0;
  }
</style>
