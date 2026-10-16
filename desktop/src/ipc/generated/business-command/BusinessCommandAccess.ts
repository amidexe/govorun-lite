// @generated from shared/core serde JSON Schema; do not edit.
export interface BusinessCommandAccess {
  configured: boolean;
  entitlementActive: boolean;
  networkAvailable: boolean | null;
  processingReady: boolean;
  selected: boolean;
  serviceAvailable: boolean;
  sessionCurrent: boolean;
  storageAvailable: boolean;
  termsRequired: boolean;
}
