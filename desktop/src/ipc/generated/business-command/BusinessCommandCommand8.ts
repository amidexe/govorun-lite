// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandDocument } from './BusinessCommandDocument';
import type { BusinessCommandFlow2 } from './BusinessCommandFlow2';
import type { BusinessCommandLocalStatistics } from './BusinessCommandLocalStatistics';
import type { BusinessCommandMode } from './BusinessCommandMode';
import type { BusinessCommandScope2 } from './BusinessCommandScope2';
import type { BusinessCommandStage2 } from './BusinessCommandStage2';
export type BusinessCommandCommand8 = { "content": BusinessCommandDocument; "history": boolean; "kind": "enrollment"; "other_account_history": boolean; "remote_data": boolean | null; "requested": boolean; "statistics": BusinessCommandLocalStatistics; "stored": BusinessCommandMode | null } | { "kind": "start"; "scope": BusinessCommandScope2 } | { "flow": BusinessCommandFlow2; "kind": "advance"; "scope": BusinessCommandScope2 | null; "stage": BusinessCommandStage2; "succeeded": boolean } | { "failures": number; "jitter": number; "kind": "schedule"; "pending": boolean; "succeeded": boolean };
