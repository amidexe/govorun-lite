<script lang="ts">
  import { useDesktopNavigation } from '../../shared/state/desktop-navigation';
  import { useBilling } from '../../features/billing/state.svelte';
  import SubscriptionPanel from '../billing/SubscriptionPanel.svelte';
  import Icon from '../../shared/ui/Icon.svelte';
  const navigate = useDesktopNavigation();
  let { expanded = false }: { expanded?: boolean } = $props();
  const store = useBilling();
</script>

{#if expanded}<SubscriptionPanel {store} />
{:else}<section class="surface subscription-summary">
    <div>
      <Icon name="ph_credit_card" /><span
        ><h2>Подписка и лимиты</h2>
        <p>
          {store.billing.value?.planTitle ??
            (store.billing.failed ? 'Не удалось получить сведения' : 'Загружаем…')}
        </p></span
      >
    </div>
    <button onclick={() => navigate('subscription')}>Управлять подпиской</button>
  </section>{/if}

<style>
  .subscription-summary > div {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }
  h2 {
    margin-bottom: 3px;
  }
</style>
