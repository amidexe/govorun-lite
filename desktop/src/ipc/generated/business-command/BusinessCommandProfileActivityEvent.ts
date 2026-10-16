// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandProfileRefreshReason } from './BusinessCommandProfileRefreshReason';
export type BusinessCommandProfileActivityEvent = { "kind": "inspect"; "reason": BusinessCommandProfileRefreshReason } | { "ancillary": boolean; "kind": "request"; "reason": BusinessCommandProfileRefreshReason } | { "kind": "confirmed" } | { "kind": "profileLoaded"; "ticket": number } | { "kind": "finished"; "succeeded": boolean; "ticket": number };
