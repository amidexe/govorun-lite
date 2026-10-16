// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandBatch } from './BusinessCommandBatch';
export type BusinessCommandCommand3 = { "kind": "availability" } | { "body": BusinessCommandBatch; "kind": "reliability" } | { "body": unknown; "kind": "start"; "request_id": string } | { "kind": "status"; "token": string } | { "kind": "reconcile"; "passes": Array<string> } | { "kind": "retire"; "passes": Array<string> } | { "kind": "outcome"; "latency_bucket": string; "outcome": string; "pass": string };
