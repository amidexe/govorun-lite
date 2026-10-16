import type { StylesSnapshot } from './model';

export class StyleAccessReturn {
  private scope: string | null = null;

  schedule(scope: string) {
    this.scope = scope;
  }

  take(value: StylesSnapshot): boolean {
    if (value.busy) return false;
    const expected = this.scope;
    this.scope = null;
    return expected === value.scope && value.resumeChoice;
  }
}

export const styleAccessReturn = new StyleAccessReturn();
