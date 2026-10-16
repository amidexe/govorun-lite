// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandRecordingAction } from './BusinessCommandRecordingAction';
export type BusinessCommandCommand5 = { "action": BusinessCommandRecordingAction; "enabled": boolean; "kind": "recording"; "mode": string | null } | { "enabled": boolean; "kind": "enable" } | { "kind": "dismissTerms" } | { "accepted_at_millis": number; "kind": "acceptTerms"; "version": number } | { "kind": "reset" };
