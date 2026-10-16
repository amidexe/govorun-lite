import type { DictionaryEntry } from './model.ts';

export function dictionaryText(entries: DictionaryEntry[]): string {
  return entries.map((entry) => `${entry.from} =${entry.to ? ` ${entry.to}` : ''}`).join('\n');
}

export function parseDictionary(text: string, previous: DictionaryEntry[]): DictionaryEntry[] {
  const reusable = new Map<string, string[]>();
  for (const entry of previous) {
    const key = JSON.stringify([entry.from, entry.to]);
    reusable.set(key, [...(reusable.get(key) ?? []), entry.id]);
  }
  const entries: DictionaryEntry[] = [];
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) continue;
    const separator = trimmed.includes(' → ') ? ' → ' : '=';
    const index = trimmed.indexOf(separator);
    if (index <= 0) throw new Error('В каждой строке нужны исходное слово и знак «=».');
    const from = trimmed.slice(0, index).trim(),
      to = trimmed.slice(index + separator.length).trim();
    if (!from || from.length > 500 || to.length > 500)
      throw new Error('Каждая часть правила должна содержать не больше 500 символов.');
    const id = reusable.get(JSON.stringify([from, to]))?.shift() ?? crypto.randomUUID();
    entries.push({ id, from, to });
  }
  if (
    entries.length > 5_000 ||
    new TextEncoder().encode(entries.map((entry) => `${entry.from}=${entry.to}`).join('\n'))
      .length > 245_760
  ) {
    throw new Error('Словарь слишком велик. Сократите число или длину правил.');
  }
  return entries;
}
