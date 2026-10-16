import { onMount } from 'svelte';
import type { useBilling } from './state.svelte';
import {
  subscriptionController,
  type SubscriptionView,
} from './application/subscription-controller';

export function useSubscription(
  current: () => ReturnType<typeof useBilling>,
  acceptPromoTerms: (request: { termsVersion: number; selection: string }) => Promise<boolean>,
) {
  let view = $state<SubscriptionView>();
  const controller = subscriptionController(
    {
      refresh: () => current().act('refresh'),
      dismiss: () => {
        void current().act('dismissPreview');
        void current().act('dismissPromo');
      },
      confirm: (action) => current().act(action, { confirmed: true }),
      activateStarter: (request) =>
        current().act('activateStarter', { ...request, confirmed: true }),
      acceptPromoTerms,
      dismissPromoAccess: (continuationId, deferred) =>
        current().act('dismissPromoAccess', { continuationId, deferred }),
      refreshPromoAccess: (continuationId) =>
        current().act('refreshPromoAccess', { continuationId }),
      visible: () => document.visibilityState === 'visible',
      onFocus(action) {
        window.addEventListener('focus', action);
        return () => window.removeEventListener('focus', action);
      },
      after(milliseconds, action) {
        const timer = setTimeout(action, milliseconds);
        return () => clearTimeout(timer);
      },
    },
    (next) => {
      view = next;
    },
  );
  view = controller.view;
  $effect(() =>
    controller.update(current().billing.value, current().busy, current().billing.failed),
  );
  onMount(() => {
    controller.start();
    return () => controller.dispose();
  });
  return {
    controller,
    get view() {
      return view!;
    },
  };
}
