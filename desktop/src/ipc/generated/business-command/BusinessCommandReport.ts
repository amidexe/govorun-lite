// @generated from shared/core serde JSON Schema; do not edit.
import type { BusinessCommandDay2 } from './BusinessCommandDay2';
import type { BusinessCommandRequestCount } from './BusinessCommandRequestCount';
export interface BusinessCommandReport {
  days: Array<BusinessCommandDay2>;
  requests: Record<string, BusinessCommandRequestCount>;
  scopeId: string;
}
