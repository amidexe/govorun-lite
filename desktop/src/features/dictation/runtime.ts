export type IndicatorIssue = 'captureUnavailable' | 'processingFailed' | 'timerUnavailable';

export interface RuntimeReadiness {
  model: 'unprepared' | 'loading' | 'ready' | 'standby' | 'failed' | 'stopped';
  microphone: boolean;
  microphoneAvailable?: boolean;
  accessibility: boolean;
  hotkey: boolean;
}
export interface RuntimeSnapshot {
  protocolVersion: 1;
  platform: 'macos' | 'windows';
  phase: 'idle' | 'starting' | 'recording' | 'processing' | 'fallback';
  capabilities: string[];
  styleStatus?: 'idle' | 'processing' | 'failed';
  issue?: IndicatorIssue | null;
  readiness?: RuntimeReadiness | null;
  hasLocalDictation?: boolean | null;
  displayName?: string;
  storageUnavailable?: boolean;
}

export function runtimeSnapshot(value: unknown): RuntimeSnapshot {
  if (!value || typeof value !== 'object') throw new Error('Invalid native snapshot');
  const snapshot = value as Partial<RuntimeSnapshot>;
  if (
    snapshot.protocolVersion !== 1 ||
    !['macos', 'windows'].includes(snapshot.platform ?? '') ||
    !['idle', 'starting', 'recording', 'processing', 'fallback'].includes(snapshot.phase ?? '') ||
    !Array.isArray(snapshot.capabilities) ||
    !snapshot.capabilities.every((item) => typeof item === 'string')
  ) {
    throw new Error('Incompatible native service');
  }
  if (snapshot.styleStatus !== undefined && !['idle', 'processing', 'failed'].includes(snapshot.styleStatus))
    throw new Error('Invalid style status');
  if (
    snapshot.issue != null &&
    !['captureUnavailable', 'processingFailed', 'timerUnavailable'].includes(snapshot.issue)
  )
    throw new Error('Invalid native issue');
  if (snapshot.readiness != null) {
    const readiness = snapshot.readiness;
    if (
      !['unprepared', 'loading', 'ready', 'standby', 'failed', 'stopped'].includes(
        readiness.model,
      ) ||
      typeof readiness.microphone !== 'boolean' ||
      (readiness.microphoneAvailable !== undefined &&
        typeof readiness.microphoneAvailable !== 'boolean') ||
      typeof readiness.accessibility !== 'boolean' ||
      typeof readiness.hotkey !== 'boolean'
    )
      throw new Error('Invalid native readiness');
  }
  if (snapshot.hasLocalDictation != null && typeof snapshot.hasLocalDictation !== 'boolean')
    throw new Error('Invalid setup history');
  if (snapshot.storageUnavailable !== undefined && snapshot.storageUnavailable !== true)
    throw new Error('Invalid storage state');
  if (
    snapshot.displayName !== undefined &&
    (typeof snapshot.displayName !== 'string' || snapshot.displayName.length > 64)
  )
    throw new Error('Invalid application name');
  return snapshot as RuntimeSnapshot;
}
