// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandOutcome } from './BusinessCommandOutcome';
import type { BusinessCommandTrigger } from './BusinessCommandTrigger';
export type BusinessCommandEvent10 = { "delay_millis": number; "join": boolean; "kind": "request"; "trigger": BusinessCommandTrigger } | { "kind": "complete"; "outcome": BusinessCommandOutcome; "retry_after_millis": number | null; "ticket": number; "wire_attempted": boolean } | { "kind": "tick"; "ticket": number } | { "kind": "cancel" };
