// @generated from shared/core serde JSON Schema; do not edit.
export interface BusinessCommandRequestSnapshot {
  blindedMessage: string;
  encodedMessage: string;
  exponent: string;
  inverse: string;
  keyId: string;
  keyNotAfterMillis: number;
  keyNotBeforeMillis: number;
  modulus: string;
  preparedMessage: string;
}
