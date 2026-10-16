/** Validate an untrusted enum without converting arrays or objects to strings. */
export function stringEnum<Value extends string>(
  value: unknown,
  allowed: readonly Value[],
): value is Value {
  return typeof value === 'string' && allowed.some((candidate) => candidate === value);
}
