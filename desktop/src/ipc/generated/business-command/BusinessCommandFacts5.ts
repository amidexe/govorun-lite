// @generated from shared/core serde JSON Schema; do not edit.
export interface BusinessCommandFacts5 {
  authorized: boolean;
  availability: string;
  clientTermsVersion: number;
  consentAccepted: boolean;
  enabled: boolean;
  entitled: boolean;
  generation: number;
  localTermsVersion: number;
  paused: boolean;
  permissionsReady: boolean;
  presetCount: number;
  processingReadiness: string;
  processingReady: boolean;
  profileFresh: boolean;
  selfHosted: boolean;
  selfHostedReady: boolean;
  signedIn: boolean;
  starterLocallyExhausted: boolean;
  storageAvailable: boolean;
  termsMatch: boolean;
}
