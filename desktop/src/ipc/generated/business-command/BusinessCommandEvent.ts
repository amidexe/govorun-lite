// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandDocument } from './BusinessCommandDocument';
export type BusinessCommandEvent = { "force": boolean; "kind": "start" } | { "configured": boolean; "document": BusinessCommandDocument; "etag": string; "kind": "read"; "not_modified": boolean } | { "document": BusinessCommandDocument; "etag": string; "kind": "written" } | { "kind": "failed"; "status": number | null };
