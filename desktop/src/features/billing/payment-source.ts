export type PaymentSource = 'card' | 'sbp';
export function paymentSources(value: unknown): value is PaymentSource[] {
  return (
    Array.isArray(value) &&
    value.length <= 2 &&
    new Set(value).size === value.length &&
    value.every((kind) => kind === 'card' || kind === 'sbp')
  );
}
export function validSBPBindingURL(value: unknown): value is string {
  if (typeof value !== 'string' || value.length > 2048 || !value.startsWith('https://sub.nspk.ru/'))
    return false;
  try {
    const url = new URL(value);
    return (
      url.protocol === 'https:' &&
      url.hostname === 'sub.nspk.ru' &&
      !url.port &&
      !url.username &&
      !url.password &&
      !url.hash &&
      url.pathname.length > 1 &&
      ((!value.includes('?') && /^[A-Za-z0-9]{32}$/.test(url.pathname.slice(1))) ||
        (url.searchParams.getAll('type').length === 1 && url.searchParams.get('type') === '03'))
    );
  } catch {
    return false;
  }
}
