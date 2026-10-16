// @generated from shared/core serde JSON Schema; do not edit.
export interface DictionaryEntry {
  from: string;
  id: string;
  to: string;
}
export interface DictionarySnapshot {
  enabled: boolean;
  entries: Array<DictionaryEntry>;
  revision: string;
}
export interface DictionaryToggle {
  enabled: boolean;
  revision: string;
}
