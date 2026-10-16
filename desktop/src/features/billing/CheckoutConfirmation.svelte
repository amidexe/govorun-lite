<script lang="ts">
  import { untrack } from 'svelte';
  import type { PaymentSource } from './payment-source';
  import type { CheckoutPreview } from './model';
  import Modal from '../../shared/ui/Modal.svelte';
  import PaymentSourceChoice from './PaymentSourceChoice.svelte';
  let {
    preview,
    busy,
    message,
    onclose,
    onconfirm,
    onlegal,
    onmethod,
  }: {
    preview: CheckoutPreview;
    busy: boolean;
    message: string | null;
    onclose: () => void;
    onconfirm: (email: string | null) => void;
    onlegal: (document: string) => void;
    onmethod: (method: PaymentSource) => void;
  } = $props();
  let email = $state(untrack(() => preview.email ?? '')),
    accepted = $state(false);
  let needsEmail = $derived(preview.receiptContactNeeded);
  let valid = $derived(!needsEmail || /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.trim()));
</script>

<Modal
  title={preview.title}
  onclose={() => {
    if (!busy) onclose();
  }}
>
  <div class="details">
    <strong>{preview.planTitle}</strong>
    <p>{preview.trialHeadline}</p>
    <strong>{preview.amountDueNow}</strong>
    <p>{preview.firstCharge}</p>
    <p>{preview.renewal}</p>
    {#if preview.handoff}<p>{preview.handoff}</p>{/if}
    <p>{preview.recurringDisclosure}</p>
    {#if preview.availablePaymentMethods?.some((kind) => kind !== preview.paymentMethodKind)}
      <PaymentSourceChoice
        selected={preview.paymentMethodKind ?? 'card'}
        available={preview.availablePaymentMethods ?? ['card']}
        {busy}
        onselect={(kind) => {
          accepted = false;
          onmethod(kind);
        }}
      />
    {/if}
    {#if preview.receiptContactNeeded}
      <label
        >Почта для чеков и уведомлений<input
          aria-label="Почта для чеков и уведомлений"
          type="email"
          maxlength="64"
          bind:value={email}
          disabled={busy}
          autocomplete="email"
        /></label
      >
      <small>Проверьте, что у вас есть доступ к этой почте.</small>
    {/if}
    {#if preview.hasLegalDocuments}<div class="links">
        <button class="quiet" onclick={() => onlegal('subscriptionTerms')}
          >Условия и политика</button>
      </div>{/if}
    <label class="consent"
      ><input type="checkbox" bind:checked={accepted} disabled={busy} /><span
        >{preview.consent}</span
      ></label
    >
    {#if message}<p class="error" role="alert">{message}</p>{/if}
    {#if !preview.actionEnabled}<p>Оформление подписки сейчас недоступно.</p>{/if}
  </div>
  {#snippet actions()}<button disabled={busy} onclick={onclose}>Отмена</button><button
      class="primary"
      disabled={busy || !accepted || !valid || !preview.actionEnabled}
      onclick={() => onconfirm(needsEmail ? email.trim() : null)}
      >{busy ? 'Проверяем…' : preview.actionTitle}</button
    >{/snippet}
</Modal>

<style>
  .details {
    display: grid;
    gap: 10px;
  }
  label:not(.consent) {
    display: grid;
    gap: 5px;
  }
  .consent {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 11px;
  }
  .consent input {
    margin-top: 2px;
    flex-shrink: 0;
  }
  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .links button {
    font-size: 11px;
    min-height: 26px;
    padding: 3px 6px;
    color: var(--accent-text);
  }
</style>
