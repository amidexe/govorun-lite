// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandWireValues } from './BusinessCommandWireValues';
export interface BusinessCommandDocument2 {
  configured: boolean;
  revision: number;
  schemaVersion: number;
  updatedAt: string | null;
  values: BusinessCommandWireValues;
}
