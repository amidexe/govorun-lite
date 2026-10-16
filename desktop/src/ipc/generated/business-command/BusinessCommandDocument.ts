// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandDictionary } from './BusinessCommandDictionary';
import type { BusinessCommandStyle } from './BusinessCommandStyle';
export interface BusinessCommandDocument {
  customStyles: Array<BusinessCommandStyle>;
  dictionary: BusinessCommandDictionary;
  schemaVersion: number;
  selectedCustomStyleId: string;
}
