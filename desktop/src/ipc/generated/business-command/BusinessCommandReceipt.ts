// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandDeviceDay } from './BusinessCommandDeviceDay';
import type { BusinessCommandTotal } from './BusinessCommandTotal';
export interface BusinessCommandReceipt {
  account: string;
  days: Record<string, BusinessCommandDeviceDay>;
  epoch: number;
  total: BusinessCommandTotal;
}
