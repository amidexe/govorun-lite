// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandFailureKind } from './BusinessCommandFailureKind';
export interface BusinessCommandFailure {
  code: string | null;
  kind: BusinessCommandFailureKind;
  status: number | null;
}
