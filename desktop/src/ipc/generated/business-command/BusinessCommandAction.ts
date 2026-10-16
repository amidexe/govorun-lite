// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandEvent2 } from './BusinessCommandEvent2';
import type { BusinessCommandReceipt } from './BusinessCommandReceipt';
import type { BusinessCommandResponse } from './BusinessCommandResponse';
export type BusinessCommandAction = { "kind": "inspect" } | { "kind": "countWords"; "text": string } | { "event": BusinessCommandEvent2; "kind": "record"; "today": string } | { "account": string | null; "kind": "reset" } | { "account": string; "kind": "prepare"; "today": string } | { "account": string; "epoch": number; "kind": "deleteAccepted" } | { "account": string; "kind": "accept"; "receipt": BusinessCommandReceipt; "response": BusinessCommandResponse };
