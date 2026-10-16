// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandDocument } from './BusinessCommandDocument';
import type { BusinessCommandWrite } from './BusinessCommandWrite';
export interface BusinessCommandState {
  base: BusinessCommandDocument;
  checkedAt: number | null;
  etag: string;
  inFlight: BusinessCommandWrite | null;
  schemaVersion: number;
}
