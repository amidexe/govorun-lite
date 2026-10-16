// @generated from shared/core serde JSON Schema; do not edit.
export interface AccountDevice {
  confirmation: string;
  current: boolean;
  id: string;
  name: string;
  subtitle: string;
}
export interface AccountProvider {
  enabled: boolean;
  id: string;
}
export interface AccountSnapshot {
  busy: boolean;
  deviceLimit: number;
  devices: Array<AccountDevice>;
  displayName: string | null;
  messageCode: string | null;
  providerNoticeCode: string | null;
  providers: Array<AccountProvider>;
  replacingDevice: boolean;
  scope: string;
  status: string;
  storage: AccountStorage;
  syncMode: string;
}
export interface AccountStorage {
  available: boolean;
  state: string;
}
