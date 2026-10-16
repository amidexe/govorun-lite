// @generated from shared/core serde JSON Schema; do not edit.
export interface UpdateSnapshot {
  channel: UpdatesChannel;
  channelSelectable: boolean;
  dismissed: boolean;
  error: string | null;
  release: UpdatesRelease | null;
  status: string;
  transfer: UpdatesUpdateTransfer | null;
}
export type UpdatesChannel = "stable" | "test";
export interface UpdatesRelease {
  notes: string;
  version: string;
}
export interface UpdatesUpdateTransfer {
  downloaded: number;
  total: number;
}
