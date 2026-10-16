// @generated from shared/core serde JSON Schema; do not edit.
export interface HotkeySnapshot {
  canCapture: boolean;
  cancel: Array<string>;
  captureID: string | null;
  dictation: Array<string>;
  inputMode?: string;
  message: string | null;
  plain: Array<string>;
  preview: Array<string>;
  target: HotkeysTarget | null;
}
export type HotkeysTarget = "dictation" | "plain" | "cancel";
