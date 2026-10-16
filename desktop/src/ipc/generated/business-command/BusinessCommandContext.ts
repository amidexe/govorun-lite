// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandOptions } from './BusinessCommandOptions';
import type { BusinessCommandState2 } from './BusinessCommandState2';
export interface BusinessCommandContext {
  builtInOptions: Record<string, BusinessCommandOptions>;
  currentOptions: BusinessCommandOptions;
  currentOptionsStyle: string;
  state: BusinessCommandState2;
}
