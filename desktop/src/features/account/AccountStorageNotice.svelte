<script lang="ts">
  import type { AccountSnapshot } from './model';
  import Modal from '../../shared/ui/Modal.svelte';

  let {
    storage,
    scope,
    busy,
    onrecover,
    onrestart,
    embedded = false,
  }: {
    storage: AccountSnapshot['storage'];
    scope: string;
    busy: boolean;
    onrecover: () => Promise<void>;
    onrestart?: () => void;
    embedded?: boolean;
  } = $props();
  let confirming = $state(false);
  let retryScope = $state<string | null>(null);
  let submitted = $state(false);
  $effect(() => {
    if (storage.available || (retryScope !== null && retryScope !== scope)) {
      confirming = false;
      retryScope = null;
      submitted = false;
    }
  });
  async function retry() {
    retryScope = scope;
    submitted = false;
    await onrecover();
    submitted = true;
  }
</script>

<section class:surface={!embedded} class:storage-notice={embedded}>
  <p role="status">{storage.title}</p>
  <button class="primary" disabled={busy} onclick={retry}
    >{busy ? (storage.state === 'permissionRequired' ? 'Восстанавливаем доступ…' : 'Проверяем…') : storage.action}</button
  >
  {#if submitted && storage.state === 'permissionRequired' && !busy && onrestart}<button
      class="quiet"
      onclick={() => {
        confirming = true;
      }}>Не удаётся восстановить?</button
    >{/if}
</section>

{#if confirming}<Modal
    title="Войти в Говорун заново?"
    onclose={() => {
      confirming = false;
    }}
  >
    <p>Настройки, словарь и статистика сохранятся.</p>
    {#snippet actions()}
      <button
        disabled={busy}
        onclick={() => {
          confirming = false;
        }}>Отмена</button
      >
      <button
        class="primary"
        disabled={busy}
        onclick={() => {
          confirming = false;
          onrestart?.();
        }}>Войти заново</button
      >
    {/snippet}
  </Modal>{/if}

<style>
  .storage-notice {
    display: grid;
    justify-items: start;
    gap: 8px;
  }
</style>
