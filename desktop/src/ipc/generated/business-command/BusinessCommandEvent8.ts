// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandFailure2 } from './BusinessCommandFailure2';
export type BusinessCommandEvent8 = { "kind": "begin"; "remaining_millis": number } | { "body": string; "kind": "response"; "now_millis": number; "remaining_millis": number; "retry_after": string | null; "status": number } | { "failure": BusinessCommandFailure2; "kind": "transport"; "remaining_millis": number };
