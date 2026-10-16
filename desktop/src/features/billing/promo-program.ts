import type { PromoProgram, BillingStorefront } from '../../ipc/generated/billing';
export type { PromoProgram, BillingStorefront } from '../../ipc/generated/billing';

export function validPromoProgram(value: unknown): value is PromoProgram {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const item = value as Record<string, unknown>;
  return (
    ['disabled', 'scheduled', 'active', 'paused', 'ended'].includes(String(item.state)) &&
    ['visible', 'canActivate', 'participated', 'enrollmentOpen'].every(
      (key) => typeof item[key] === 'boolean',
    ) &&
    ['programVersion', 'accessMode', 'message'].every(
      (key) => typeof item[key] === 'string' && item[key].length <= 12000,
    ) &&
    ['accessDays', 'dailyLimit', 'periodLimit', 'periodDays'].every(
      (key) => Number.isSafeInteger(item[key]) && Number(item[key]) >= 0,
    ) &&
    (item.endsAt === null ||
      (typeof item.endsAt === 'string' && Number.isFinite(Date.parse(item.endsAt)))) &&
    (!item.canActivate ||
      (item.state === 'active' &&
        item.visible === true &&
        item.enrollmentOpen === true &&
        !item.participated &&
        !!item.programVersion))
  );
}

export function promoProgramDescription(program: PromoProgram): string {
  if (program.state !== 'active')
    return program.message
      ? ''
      : ({ paused: 'Приостановлено', ended: 'Завершено', scheduled: 'Скоро', disabled: '' } as Record<string, string>)[program.state] ?? '';
  if (program.endsAt) {
    const date = new Intl.DateTimeFormat('ru-RU', {
      day: 'numeric',
      month: 'long',
      year: 'numeric',
    }).format(new Date(program.endsAt));
    return `Действует до ${date}`;
  }
  if (program.accessMode === 'from_activation' && program.accessDays > 0)
    return `Срок — ${program.accessDays} дней после подключения`;
  return '';
}
