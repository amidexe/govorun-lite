// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandEvent3 } from './BusinessCommandEvent3';
import type { BusinessCommandFlow3 } from './BusinessCommandFlow3';
import type { BusinessCommandIntent } from './BusinessCommandIntent';
import type { BusinessCommandState3 } from './BusinessCommandState3';
import type { BusinessCommandValues } from './BusinessCommandValues';
export interface BusinessCommandCommand10 {
  event: BusinessCommandEvent3;
  flow: BusinessCommandFlow3 | null;
  intents: Array<BusinessCommandIntent>;
  local: BusinessCommandValues;
  state: BusinessCommandState3 | null;
}
