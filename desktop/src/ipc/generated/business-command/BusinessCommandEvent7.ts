// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandFailure2 } from './BusinessCommandFailure2';
export type BusinessCommandEvent7 = { "kind": "begin"; "remaining_millis": number } | { "kind": "response"; "now_millis": number; "remaining_millis": number; "retry_after": string | null; "status": number } | { "failure": BusinessCommandFailure2; "kind": "transport"; "remaining_millis": number } | { "elapsed_millis": number; "kind": "processing"; "remaining_millis": number };
