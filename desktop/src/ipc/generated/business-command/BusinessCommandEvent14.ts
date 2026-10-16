// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandDay2 } from './BusinessCommandDay2';
import type { BusinessCommandReport } from './BusinessCommandReport';
import type { BusinessCommandRequestCount } from './BusinessCommandRequestCount';
import type { BusinessCommandScope3 } from './BusinessCommandScope3';
export type BusinessCommandEvent14 = { "kind": "validate" } | { "day": string | null; "kind": "record"; "scopes": Array<string | null> } | { "admitted_scope": string | null; "day": string | null; "kind": "begin"; "lease_id": string; "scopes": Array<string | null> } | { "consumed": boolean; "kind": "settle"; "lease_id": string } | { "device_days": Record<string, number>; "device_requests": Record<string, BusinessCommandRequestCount>; "kind": "observe"; "protected_scopes": Array<string>; "scope": BusinessCommandScope3 } | { "days": Array<BusinessCommandDay2>; "kind": "acknowledge"; "report": BusinessCommandReport } | { "kind": "inspect"; "scope": BusinessCommandScope3 } | { "day": string | null; "kind": "outstanding"; "scope_id": string };
