<script lang="ts">
  import Modal from '../../shared/ui/Modal.svelte';
  import type { PromoPreview } from './model';
  import type { ActionArguments, BillingActions } from '../../ipc/actions';

  let {
    entryOpen = $bindable(false),
    code = $bindable(''),
    offerId,
    promo,
    message,
    busy,
    error,
    act,
  }: {
    entryOpen: boolean;
    code: string;
    offerId: string;
    promo: PromoPreview | null;
    message: string | null;
    busy: boolean;
    error: string | null;
    act: (...args: ActionArguments<BillingActions>) => Promise<boolean>;
  } = $props();
</script>

{#if entryOpen}<Modal
    title="Промокод"
    onclose={() => {
      if (!busy) entryOpen = false;
    }}
  >
    <label
      >Введите полученный код<input
        class="code"
        aria-label="Промокод"
        maxlength="128"
        bind:value={code}
        disabled={busy}
      /></label
    >
    {#if error || message}<p class="error" role="alert">
        {error ?? message}
      </p>{/if}
    {#snippet actions()}<button
        disabled={busy}
        onclick={() => {
          entryOpen = false;
        }}>Отмена</button
      ><button
        class="primary"
        disabled={busy || !code.trim()}
        onclick={() => act('previewPromo', { code: code.trim(), offerId: offerId || null })}
        >Проверить</button
      >{/snippet}
  </Modal>{/if}
{#if promo}<Modal
    title={promo.title}
    onclose={() => {
      if (!busy) void act('dismissPromo');
    }}
  >
    <p>{promo.body}</p>
    {#if error || message}<p class="error" role="alert">
        {error ?? message}
      </p>{/if}
    {#snippet actions()}<button disabled={busy} onclick={() => act('dismissPromo')}>Отмена</button
      ><button
        class="primary"
        disabled={busy}
        onclick={() => {
          if (promo) void act('confirmPromo', { previewId: promo.id, confirmed: true });
        }}>{promo?.checkoutRequired ? 'Посмотреть условия' : 'Подключить'}</button
      >{/snippet}
  </Modal>{/if}

<style>
  .code {
    display: block;
    width: 100%;
    margin-top: 8px;
  }
</style>
