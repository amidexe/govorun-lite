import type { BillingSnapshot } from './model';

export function subscriptionOverview(value: BillingSnapshot) {
  const state = value.status.fresh ? value.status.state : undefined;
  const returned = state === 'refunded';
  const paid = value.hasAccess && ['paid', 'trial'].includes(value.accessType ?? '') && !returned;
  const stalePaidAccess = returned && ['paid', 'trial'].includes(value.accessType ?? '');
  const status =
    returned || !value.status.fresh
      ? null
      : ((
          {
            active: 'Подписка активна',
            trialing: 'Пробный период',
            cancelled: 'Автопродление отключено',
            expired: 'Подписка завершена',
            grace: 'Ожидаем оплату продления',
          } as Record<string, string>
        )[state ?? ''] ?? null);
  return {
    title: stalePaidAccess ? 'Лимиты' : paid ? `Подписка «${value.planTitle}»` : value.planTitle,
    status,
    payment: paid && value.status.fresh ? value.status.payment : null,
    showBalances: !stalePaidAccess,
    paid,
    confirmedPurchase: paid && value.status.fresh && state === 'active',
  };
}
