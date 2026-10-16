// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandAuthority } from './BusinessCommandAuthority';
export interface BusinessCommandSnapshot2 {
  authority: BusinessCommandAuthority;
  logoutPending: boolean;
  profileFresh: boolean;
  sessionGeneration: number;
  signedIn: boolean;
  storageAvailable: boolean;
}
