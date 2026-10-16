export type EmojiMode = NonNullable<ApplicationRule['emojiMode']>;
import type { ApplicationChoice, ApplicationRule } from '../../ipc/generated/styles';
export type { ApplicationChoice, ApplicationRule } from '../../ipc/generated/styles';
function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('invalid_application_rule');
  return value as Record<string, unknown>;
}
function identity(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= 512;
}
export function applicationChoices(value: unknown): ApplicationChoice[] {
  if (
    !Array.isArray(value) ||
    value.length > 4096 ||
    !value.every((value) => {
      const item = record(value);
      return identity(item.id) && identity(item.name);
    }) ||
    new Set(value.map((item) => item.id)).size !== value.length
  )
    throw new Error('invalid_application_catalog');
  return value as ApplicationChoice[];
}
export function applicationRules(value: unknown): ApplicationRule[] {
  const items = applicationChoices(value);
  if (
    !items.every((value) => {
      const item = record(value);
      return (
        (item.style === null || identity(item.style)) &&
        (item.emojiMode === null ||
          ['off', 'restrained', 'lively', 'bright'].includes(String(item.emojiMode))) &&
        (item.reducedPunctuation === null || typeof item.reducedPunctuation === 'boolean')
      );
    })
  )
    throw new Error('invalid_application_rule');
  return items as ApplicationRule[];
}
