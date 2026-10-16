// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandFlight } from './BusinessCommandFlight';
import type { BusinessCommandProfileRefreshCadence } from './BusinessCommandProfileRefreshCadence';
export interface BusinessCommandProfileActivity {
  cadence: BusinessCommandProfileRefreshCadence;
  flight: BusinessCommandFlight | null;
  generation: number | null;
  sequence: number;
}
