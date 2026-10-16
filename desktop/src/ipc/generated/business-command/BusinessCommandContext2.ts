// @generated from shared/core serde JSON Schema; do not edit.
export interface BusinessCommandContext2 {
  backgroundAllowed: boolean;
  explicitDemand: boolean;
  issuanceCountersObserved: boolean;
  issuanceRemainingToday: number;
  issuanceRemainingTotal: number;
  pendingIssuanceCredits: number;
  replacementReceipts: number;
  reserveCredits: number;
  retiringCredits: number;
}
