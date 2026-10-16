// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandDocument2 } from './BusinessCommandDocument2';
export type BusinessCommandEvent3 = { "kind": "start" } | { "document": BusinessCommandDocument2; "kind": "document" } | { "kind": "failed"; "status": number | null };
