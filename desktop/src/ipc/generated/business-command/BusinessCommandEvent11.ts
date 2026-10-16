// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandOutcome2 } from './BusinessCommandOutcome2';
import type { BusinessCommandTrigger2 } from './BusinessCommandTrigger2';
export type BusinessCommandEvent11 = { "join": boolean; "kind": "request"; "trigger": BusinessCommandTrigger2 } | { "kind": "complete"; "outcome": BusinessCommandOutcome2; "retry_after_millis": number | null; "status": number | null; "ticket": number } | { "kind": "tick"; "ticket": number } | { "kind": "cancel" };
