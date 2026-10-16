// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandDocument } from './BusinessCommandDocument';
import type { BusinessCommandEvent } from './BusinessCommandEvent';
import type { BusinessCommandFlow } from './BusinessCommandFlow';
import type { BusinessCommandState } from './BusinessCommandState';
export interface BusinessCommandCommand6 {
  event: BusinessCommandEvent;
  flow: BusinessCommandFlow | null;
  local: BusinessCommandDocument;
  nowMillis: number;
  state: BusinessCommandState | null;
}
