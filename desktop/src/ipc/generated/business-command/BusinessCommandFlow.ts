// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandStage } from './BusinessCommandStage';
export interface BusinessCommandFlow {
  conditional: boolean;
  stage: BusinessCommandStage;
  writes: number;
}
