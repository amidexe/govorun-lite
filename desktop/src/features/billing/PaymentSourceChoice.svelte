<script lang="ts">
  import type { PaymentSource } from './payment-source';
  import Icon from '../../shared/ui/Icon.svelte';
  import sbpLogo from '../../assets/sbp.svg';
  const sourceOrder = ['sbp', 'card'] as const;
  let {
    selected,
    available,
    busy,
    onselect,
  }: {
    selected: PaymentSource;
    available: PaymentSource[];
    busy: boolean;
    onselect: (kind: PaymentSource) => void;
  } = $props();
</script>

<fieldset disabled={busy}>
  <legend>Способ оплаты</legend>
  <div class="choices">
    {#each sourceOrder.filter((kind) => available.includes(kind)) as kind}
      <label>
        <span class="source-icon">
          {#if kind === 'sbp'}<img src={sbpLogo} alt="" width="21" height="24" />
          {:else}<Icon name="ph_credit_card" size={24} />{/if}
        </span>
        <span class="copy">{kind === 'sbp' ? 'Система быстрых платежей' : 'Банковская карта'}</span>
        <input
          type="radio"
          name="payment-source"
          value={kind}
          checked={selected === kind}
          onchange={() => {
            if (selected !== kind) onselect(kind);
          }}
        />
      </label>
    {/each}
  </div>
</fieldset>

<style>
  fieldset {
    border: 0;
    margin: 4px 0;
    padding: 0;
    min-width: 0;
  }
  legend {
    font-weight: 600;
    padding: 0;
    margin-bottom: 9px;
  }
  .choices {
    display: grid;
  }
  label {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 8px 0;
    cursor: pointer;
  }
  input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
  }
  fieldset:disabled label {
    cursor: default;
    opacity: 0.65;
  }
  .source-icon {
    width: 24px;
    height: 24px;
    flex-shrink: 0;
    display: grid;
    place-items: center;
  }
  .copy {
    flex: 1;
    min-width: 0;
    font-size: 13px;
  }
  input {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    margin: 0;
    accent-color: var(--accent);
  }
</style>
