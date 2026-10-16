// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandDeviceDay } from './BusinessCommandDeviceDay';
import type { BusinessCommandResponse } from './BusinessCommandResponse';
import type { BusinessCommandSnapshot } from './BusinessCommandSnapshot';
export type BusinessCommandCommand11 = { "current": BusinessCommandSnapshot; "kind": "accept"; "response": BusinessCommandResponse } | { "acknowledged": Record<string, number>; "kind": "prepare"; "local_days": Array<BusinessCommandDeviceDay>; "revisions": Record<string, number>; "today": string };
