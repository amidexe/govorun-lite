// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandDocument } from './BusinessCommandDocument';
export type BusinessCommandCommand13 = { "body": string; "etag": string | null; "fallback": string; "kind": "readResponse"; "status": number } | { "document": BusinessCommandDocument; "kind": "validate" } | { "base": BusinessCommandDocument; "kind": "merge"; "local": BusinessCommandDocument; "remote": BusinessCommandDocument };
