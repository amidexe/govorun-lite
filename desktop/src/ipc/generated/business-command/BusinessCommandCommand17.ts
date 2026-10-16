// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandMarker } from './BusinessCommandMarker';
import type { BusinessCommandStreamEvent } from './BusinessCommandStreamEvent';
import type { BusinessCommandStreamState } from './BusinessCommandStreamState';
export type BusinessCommandCommand17 = { "kind": "constants" } | { "fallback": boolean; "has_outcome_pass": boolean; "kind": "prepare"; "latency_known": boolean; "marker": BusinessCommandMarker; "original": Array<number>; "text": Array<number> } | { "event": BusinessCommandStreamEvent; "kind": "stream"; "marker": BusinessCommandMarker; "state": BusinessCommandStreamState | null };
