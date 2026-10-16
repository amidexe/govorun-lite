// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandStoredOptions } from './BusinessCommandStoredOptions';
import type { BusinessCommandStoredPreset } from './BusinessCommandStoredPreset';
export interface BusinessCommandStored {
  activeId: string | null;
  instruction: string;
  legacyOptions: BusinessCommandStoredOptions;
  optionsByStyle: Record<string, BusinessCommandStoredOptions>;
  presets: Array<BusinessCommandStoredPreset> | null;
  selectedStyle: string | null;
}
