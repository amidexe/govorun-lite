import type { BillingSnapshot } from './model';

export function cancellationCopy(status: BillingSnapshot['status']): string {
  const end = status.periodEnd
    ? `Доступ сохранится до ${status.periodEnd}.`
    : 'Доступ сохранится до конца текущего периода.';
  return (
    `Списания будут отключены, карта — отвязана. ${end}` +
    (status.trialActive ? ' Повторно получить бесплатный пробный период нельзя.' : '')
  );
}
