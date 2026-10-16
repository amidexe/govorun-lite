/** Decode a select control without weakening the command's enum type. */
export function choice<Value extends string>(value: string, allowed: readonly Value[]): Value {
  const selected = allowed.find((option) => option === value);
  if (selected === undefined) throw new Error('invalid_choice');
  return selected;
}
