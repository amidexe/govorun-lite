<script lang="ts">
  import { onMount } from 'svelte';
  import { checkoutSecondsRemaining, checkoutDeadlineCopy } from './checkout-recovery';
  let {
    expiresAt,
    canResume,
    failed,
    onresume,
    canCancel = false,
    oncancel = () => {},
    busy = false,
  }: {
    expiresAt?: string | null;
    canResume?: boolean;
    failed: boolean;
    onresume: () => void;
    canCancel?: boolean;
    oncancel?: () => void;
    busy?: boolean;
  } = $props();
  let now = $state(Date.now());
  const remaining = $derived(checkoutSecondsRemaining(expiresAt, now));
  const countdown = $derived(checkoutDeadlineCopy(remaining));
  onMount(() => {
    const timer = setInterval(() => {
      now = Date.now();
    }, 1000);
    return () => clearInterval(timer);
  });
</script>

{#if countdown}<p class="countdown">{countdown}</p>{/if}
<div class="checkout-actions">
  {#if canResume && remaining !== 0}<button disabled={failed} onclick={onresume}
      >Продолжить оплату</button
    >{/if}
  {#if canCancel}<button class="quiet" disabled={failed || busy} onclick={oncancel}
      >Отменить оформление</button
    >{/if}
</div>

<style>
  .countdown {
    margin: 8px 0 16px;
    font-size: inherit;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .checkout-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
</style>
