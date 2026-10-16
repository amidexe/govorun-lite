// @generated from shared/core serde JSON Schema; do not edit.
export interface StatisticsDay {
  date: string;
  seconds: number;
  sessions: number;
  words: number;
}
export type StatisticsMode = "evaluating" | "pending" | "enabled" | "localOnly";
export interface StatisticsReset {
  confirmed: boolean;
}
export interface StatisticsSnapshot {
  days: Array<StatisticsDay>;
  hasCloudSnapshot: boolean;
  signedIn: boolean;
  storageReadOnly: boolean;
  storageRecovered: boolean;
  syncMode: StatisticsMode;
  today: string;
  total: StatisticsTotal;
}
export interface StatisticsTotal {
  seconds: number;
  sessions: number;
  words: number;
}
