// @generated from shared/core serde JSON Schema; do not edit.
export type BusinessCommandStreamEvent = { "kind": "fragment"; "units": Array<number> } | { "had_text": boolean; "kind": "flush" } | { "had_text": boolean; "kind": "suffix" };
