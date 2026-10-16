// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandInputChanges } from './BusinessCommandInputChanges';
import type { BusinessCommandValues } from './BusinessCommandValues';
export type BusinessCommandIntent = { "changes": BusinessCommandInputChanges; "kind": "change"; "local": BusinessCommandValues } | { "kind": "replace"; "local": BusinessCommandValues } | { "after": BusinessCommandValues; "before": BusinessCommandValues; "kind": "compare" };
