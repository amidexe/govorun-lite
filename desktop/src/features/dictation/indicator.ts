import type { IndicatorIssue, RuntimeSnapshot } from './runtime';
export type { IndicatorIssue } from './runtime';
export type IndicatorStage =
  'recording' | 'recognizing' | 'checkingConnection' | 'accepted' | 'processing';
export interface IndicatorFrame {
  revision: number;
  phase: RuntimeSnapshot['phase'];
  stage: IndicatorStage;
  warning: boolean;
  issue: IndicatorIssue | null;
}
export function indicatorFrame(value: unknown): IndicatorFrame {
  if (!value || typeof value !== 'object') throw new Error('Invalid indicator frame');
  const frame = value as Partial<IndicatorFrame>;
  if (
    Object.keys(frame).sort().join(',') !== 'issue,phase,revision,stage,warning' ||
    !Number.isSafeInteger(frame.revision) ||
    (frame.revision ?? -1) < 0 ||
    !['idle', 'starting', 'recording', 'processing', 'fallback'].includes(frame.phase ?? '') ||
    !['recording', 'recognizing', 'checkingConnection', 'accepted', 'processing'].includes(
      frame.stage ?? '',
    ) ||
    typeof frame.warning !== 'boolean' ||
    !(
      frame.issue === null ||
      ['captureUnavailable', 'processingFailed', 'timerUnavailable'].includes(frame.issue ?? '')
    )
  ) {
    throw new Error('Invalid indicator frame');
  }
  return frame as IndicatorFrame;
}
export function acceptIndicator(current: IndicatorFrame, value: unknown): IndicatorFrame {
  const next = indicatorFrame(value);
  return next.revision > current.revision ? next : current;
}
export function indicatorLabel(frame: IndicatorFrame): string {
  if (frame.issue === 'captureUnavailable') return 'Не удалось начать запись';
  if (frame.issue === 'processingFailed') return 'Не удалось обработать запись';
  if (frame.issue === 'timerUnavailable') return 'Не удалось проверить ограничение записи';
  if (frame.warning) return 'Запись скоро завершится';
  if (frame.phase === 'starting') return 'Подготовка микрофона';
  if (frame.phase === 'recording') return 'Идёт запись';
  if (frame.phase === 'fallback') return 'Диктовка без улучшения';
  if (frame.phase === 'idle') return 'Запись завершена';
  return {
    recording: 'Идёт запись',
    recognizing: 'Распознаём речь',
    checkingConnection: 'Проверяем соединение',
    accepted: 'Запрос принят',
    processing: 'Улучшаем текст',
  }[frame.stage];
}

export interface RecordingIndicatorApi {
  subscribe(listener: (frame: unknown) => void): Promise<() => void>;
  snapshot(): Promise<unknown>;
  cancel(): Promise<unknown>;
}

/** Preserve the native capsule's controls through capture, processing and fallback. */
export function indicatorPresentation(phase: IndicatorFrame['phase']) {
  const capture = phase === 'starting' || phase === 'recording';
  return {
    visible: phase !== 'idle',
    bird: true,
    cancel: capture,
    dots: capture || phase === 'processing',
    shake: phase === 'fallback',
  };
}

/** Native macOS/Windows wave: 3.4 rad/s, adjacent dots 0.9 rad apart. */
export function indicatorDot(seconds: number, index: number) {
  const wave = Math.sin(seconds * 3.4 - index * 0.9);
  return { opacity: 0.66 + 0.24 * wave, scale: 0.94 + 0.12 * wave };
}
