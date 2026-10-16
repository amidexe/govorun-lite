// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandScope2 } from './BusinessCommandScope2';
import type { BusinessCommandStage2 } from './BusinessCommandStage2';
export interface BusinessCommandFlow2 {
  failed: boolean;
  scope: BusinessCommandScope2;
  stage: BusinessCommandStage2;
}
