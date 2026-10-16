export function checkoutSecondsRemaining(
  deadline: string | null | undefined,
  now: number,
): number | null {
  if (!deadline) return null;
  const end = Date.parse(deadline);
  return Number.isFinite(end) ? Math.max(0, Math.ceil((end - now) / 1000)) : null;
}

export function checkoutDeadlineCopy(seconds: number | null): string | null {
  if (seconds === null) return null;
  if (seconds === 0)
    return 'Срок попытки истёк. Ожидаем окончательный результат банка. Если платёж не прошёл, можно будет оформить заказ снова.';
  const minutes = Math.floor(seconds / 60)
    .toString()
    .padStart(2, '0');
  const remainder = (seconds % 60).toString().padStart(2, '0');
  return `Оформление доступно ещё ${minutes}:${remainder}`;
}
