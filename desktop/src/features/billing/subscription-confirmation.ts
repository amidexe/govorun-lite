import type { BillingSnapshot } from './model';
import { cancellationCopy } from './subscription-cancellation.ts';

export type ConfirmationAction =
  'activateStarter' | 'cancelRenewal' | 'cancelPlanChange' | 'removeCard';

export function subscriptionConfirmation(value: BillingSnapshot, action: ConfirmationAction) {
  const copy = {
    cancelRenewal: ['Отменить подписку?', cancellationCopy(value.status), 'Отменить подписку'],
    cancelPlanChange: ['Отменить смену тарифа?', 'Текущий тариф сохранится.', 'Отменить смену'],
    removeCard: ['Отключить способ оплаты?', cancellationCopy(value.status), 'Отключить'],
    activateStarter: [
      'Попробовать улучшение текста?',
      'Аудио остаётся на устройстве. Для улучшения распознанный текст отправляется сервису Говоруна. Бесплатный доступ не требует карты и не включает автопродление.',
      'Принять и попробовать',
    ],
  }[action];
  return {
    scope: value.scope,
    termsVersion: value.termsVersion,
    programVersion: value.promoProgram?.programVersion ?? '',
    action,
    title: copy[0],
    body: copy[1],
    actionTitle: copy[2],
  };
}
