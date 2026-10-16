import type { TextStyleOptions, StylePreset, StylesSnapshot } from '../../ipc/generated/styles';
export type { TextStyleOptions, StylePreset, StylesSnapshot } from '../../ipc/generated/styles';
import { stringEnum } from '../../shared/validation/string-enum.ts';

import {
  applicationRules,
  applicationChoices,
  type ApplicationRule,
  type ApplicationChoice,
} from './application-rules.ts';
export type StylesAction =
  | 'select'
  | 'disable'
  | 'acceptTerms'
  | 'cancel'
  | 'refresh'
  | 'documents'
  | 'savePreset'
  | 'deletePreset'
  | 'option'
  | 'loadApplications'
  | 'saveApplicationRule'
  | 'deleteApplicationRule';

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error('invalid_styles');
  return value as Record<string, unknown>;
}
function text(value: unknown, limit = 2000): value is string {
  return typeof value === 'string' && value.length <= limit;
}
function optionalText(value: unknown): boolean {
  return value === null || text(value);
}
function validOptions(value: unknown): boolean {
  const options = record(value);
  return (
    stringEnum(options.emojiMode, ['off', 'restrained', 'lively', 'bright']) &&
    typeof options.reducedPunctuation === 'boolean' &&
    typeof options.profanityFilter === 'boolean'
  );
}
export function stylesSnapshot(value: unknown): StylesSnapshot {
  const item = record(value),
    options = record(item.options);
  if (
    !text(item.scope, 160) ||
    !item.scope ||
    typeof item.busy !== 'boolean' ||
    !optionalText(item.message) ||
    !stringEnum(item.gate, [
      'ready',
      'account',
      'storage',
      'permissions',
      'terms',
      'access',
      'unavailable',
      'selfHosted',
      'selfHostedSetup',
    ]) ||
    (item.onlineServicePaused !== undefined && typeof item.onlineServicePaused !== 'boolean') ||
    (item.processingReadiness !== undefined && !stringEnum(item.processingReadiness, ['ready', 'onDemand', 'preparing', 'needsPreparation', 'unavailable'])) ||
    !optionalText(item.availability) ||
    !stringEnum(item.availabilityReason, ['ready', 'checking', 'unavailable', 'periodLimit', 'dailyLimit']) ||
    !stringEnum(item.displayState, ['disabled', 'active', 'preparing', 'saved', 'checking']) ||
    !stringEnum(item.limitReason, ['none', 'period', 'daily']) ||
    typeof item.canCreatePreset !== 'boolean' ||
    typeof item.resumeChoice !== 'boolean' ||
    typeof item.enabled !== 'boolean' ||
    (item.selected !== null && !text(item.selected, 160)) ||
    !text(item.title, 512) ||
    !Array.isArray(item.styles) ||
    item.styles.length > 20 ||
    !item.styles.every((value) => {
      const style = record(value);
      return (
        stringEnum(style.id, ['formal', 'light', 'concise']) &&
        text(style.title, 512) &&
        text(style.detail) &&
        (style.options === undefined || validOptions(style.options))
      );
    }) ||
    new Set(item.styles.map((item) => item.id)).size !== item.styles.length ||
    !Array.isArray(item.presets) ||
    item.presets.length > 1000 ||
    !item.presets.every((value) => {
      const preset = record(value);
      return (
        text(preset.id, 128) &&
        preset.id.length > 0 &&
        text(preset.name, 256) &&
        text(preset.instruction, 2000) &&
        (preset.options === undefined || validOptions(preset.options))
      );
    }) ||
    new Set(item.presets.map((item) => item.id)).size !== item.presets.length ||
    !Number.isSafeInteger(item.maxPresets) ||
    Number(item.maxPresets) < 0 || Number(item.maxPresets) > 100 ||
    !stringEnum(options.emojiMode, ['off', 'restrained', 'lively', 'bright']) ||
    typeof options.reducedPunctuation !== 'boolean' ||
    typeof options.profanityFilter !== 'boolean' ||
    !Number.isSafeInteger(item.termsVersion) ||
    Number(item.termsVersion) < 1 ||
    typeof item.termsAccepted !== 'boolean'
  ) {
    throw new Error('invalid_styles');
  }
  applicationRules(item.applicationRules);
  applicationChoices(item.applications);
  const selection = item.selected;
  if (
    selection !== null &&
    selection !== 'selfHosted' &&
    !item.styles.some((style) => style.id === selection) &&
    !item.presets.some((preset) => `custom:${preset.id}` === selection)
  )
    throw new Error('invalid_styles');
  return item as unknown as StylesSnapshot;
}
