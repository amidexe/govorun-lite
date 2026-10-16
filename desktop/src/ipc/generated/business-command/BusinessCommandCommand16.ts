// @generated from shared/core serde JSON Schema; do not edit.
export type BusinessCommandCommand16 = { "input": Array<number>; "kind": "begin" } | { "input": Array<number>; "kind": "response"; "output": Array<number> } | { "kind": "failure"; "status": number };
