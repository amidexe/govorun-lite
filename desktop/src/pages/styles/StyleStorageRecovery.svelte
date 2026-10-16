<script lang="ts">
  import { untrack } from 'svelte';
  import { useAccount } from '../../features/account/state.svelte';
  import AccountStorageNotice from '../../features/account/AccountStorageNotice.svelte';

  let { onrecovered, onrestart }: { onrecovered: () => Promise<void>; onrestart: () => void } = $props();
  const state = useAccount();
  let account = $derived(state.account.value);
  let busy = $derived(state.pending || account?.busy === true);
  $effect(() => {
    if (account?.storage.available && !account.busy)
      untrack(() => {
        void onrecovered();
      });
  });

  async function recover() {
    await state.act('recoverStorage');
    await onrecovered();
  }
</script>

<div class="storage-recovery">
  {#if account && !account.storage.available}
    <AccountStorageNotice
      storage={account.storage}
      scope={account.scope}
      {busy}
      onrecover={recover}
      onrestart={async () => {
        await state.act('restartStorage', { confirmed: true });
        if (!state.error && state.account.value?.storage.available) onrestart();
        await onrecovered();
      }}
      embedded
    />
  {:else if state.account.failed}
    <p role="alert">Не удалось проверить доступ к улучшению текста.</p>
    <button
      onclick={async () => {
        await state.account.refresh();
        await onrecovered();
      }}>Повторить проверку</button
    >
  {:else}
    <p role="status">Проверяем доступ к улучшению текста…</p>
  {/if}
  {#if state.error}<p class="error" role="alert">{state.error}</p>{/if}
</div>

<style>
  .storage-recovery {
    display: grid;
    gap: 12px;
  }
</style>
