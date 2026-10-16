// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandAccess } from './BusinessCommandAccess';
import type { BusinessCommandCommand15 } from './BusinessCommandCommand15';
import type { BusinessCommandCommand16 } from './BusinessCommandCommand16';
import type { BusinessCommandCommand17 } from './BusinessCommandCommand17';
import type { BusinessCommandEvent4 } from './BusinessCommandEvent4';
import type { BusinessCommandEvent5 } from './BusinessCommandEvent5';
import type { BusinessCommandEvent6 } from './BusinessCommandEvent6';
import type { BusinessCommandFacts2 } from './BusinessCommandFacts2';
import type { BusinessCommandPlan } from './BusinessCommandPlan';
import type { BusinessCommandSettings } from './BusinessCommandSettings';
import type { BusinessCommandState4 } from './BusinessCommandState4';
import type { BusinessCommandState5 } from './BusinessCommandState5';
import type { BusinessCommandState6 } from './BusinessCommandState6';
export type BusinessCommandCommand14 = { "command": BusinessCommandCommand15; "kind": "onlineProtocol" } | { "event": BusinessCommandEvent4; "kind": "outage"; "now_millis": number; "state": BusinessCommandState4 | null } | { "facts": BusinessCommandFacts2; "kind": "recording" } | { "command": BusinessCommandCommand16; "kind": "selfHosted" } | { "deferred": boolean; "event": BusinessCommandEvent5; "kind": "transcript"; "state": BusinessCommandState5 | null } | { "command": BusinessCommandCommand17; "kind": "delivery" } | { "event": BusinessCommandEvent6; "kind": "workflow"; "state": BusinessCommandState6 | null } | { "characters": number; "kind": "deadline" } | { "kind": "constants" } | { "kind": "plan"; "settings": BusinessCommandSettings; "units": Array<number> } | { "access": BusinessCommandAccess; "kind": "admit"; "plan": BusinessCommandPlan } | { "input": Array<number>; "kind": "response"; "output": Array<number> };
