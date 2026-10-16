// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandIntent2 } from './BusinessCommandIntent2';
import type { BusinessCommandLifetime } from './BusinessCommandLifetime';
export type BusinessCommandEvent15 = { "accept_terms": boolean; "confirmed": boolean; "kind": "begin"; "operation_id": string; "selection": string; "selection_exists": boolean; "terms_version": number } | { "intent": BusinessCommandIntent2; "kind": "complete"; "lifetime": BusinessCommandLifetime; "selection_exists": boolean } | { "generation": number; "kind": "failed"; "lifetime": BusinessCommandLifetime; "operation_id": string };
