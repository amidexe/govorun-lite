// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandStyle } from './BusinessCommandStyle';
export interface BusinessCommandState2 {
  activeId: string | null;
  instruction: string;
  presets: Array<BusinessCommandStyle>;
  selectedStyle: string;
}
