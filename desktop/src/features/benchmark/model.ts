import type { BenchmarkTiming, BenchmarkResult, BenchmarkDevice, BenchmarkSnapshot } from '../../ipc/generated/benchmark';
export type { BenchmarkTiming, BenchmarkResult, BenchmarkDevice, BenchmarkSnapshot } from '../../ipc/generated/benchmark';
import { stringEnum } from '../../shared/validation/string-enum.ts';

export type BenchmarkFailure =
  | 'samples_unavailable'
  | 'model_missing'
  | 'runtime_incompatible'
  | 'initialization_failed'
  | 'recognition_failed';
export type BenchmarkStage = 'fixtures' | 'model' | 'cold' | 'warm' | 'long';
const stages = ['fixtures', 'model', 'cold', 'warm', 'long'];
const failures = [
  'samples_unavailable',
  'model_missing',
  'runtime_incompatible',
  'initialization_failed',
  'recognition_failed',
];
const failureDetails = [
  'recognition_engine_out_of_memory',
  'recognition_engine_backend_failed',
  'recognition_engine_model_failed',
  'recognition_device_fallback',
  'recognition_devices_unavailable',
];
function object(value: unknown, keys: string[]): Record<string, unknown> {
  if (
    !value ||
    typeof value !== 'object' ||
    Array.isArray(value) ||
    Object.keys(value).length !== keys.length ||
    Object.keys(value).some((key) => !keys.includes(key))
  )
    throw new Error('invalid_benchmark');
  return value as Record<string, unknown>;
}
function validFailure(value: unknown, state: unknown): boolean {
  return value === null ? state !== 'failed' : state === 'failed' && stringEnum(value, failures);
}
function timing(value: unknown, backend: unknown): boolean {
  const item = object(value, ['milliseconds', 'backend']);
  return (
    typeof item.milliseconds === 'number' &&
    Number.isFinite(item.milliseconds) &&
    item.milliseconds > 0 &&
    item.backend === backend
  );
}
function result(value: unknown, backend: unknown): void {
  const item = object(value, [
    'suite',
    'platform',
    'model',
    'requestedBackend',
    'shortAudioMs',
    'longAudioMs',
    'loadMs',
    'cold',
    'warm',
    'longer',
  ]);
  if (
    item.suite !== 'gigaam-samples-v1' ||
    !stringEnum(item.platform, ['windows', 'macos']) ||
    item.requestedBackend !== (backend === 'cpu' ? 'cpu' : 'automatic') ||
    (backend === 'metal' && item.platform !== 'macos') ||
    (backend === 'vulkan' && item.platform !== 'windows') ||
    typeof item.model !== 'string' ||
    item.model.length > 160 ||
    item.shortAudioMs !== 11290 ||
    item.longAudioMs !== 71250 ||
    typeof item.loadMs !== 'number' ||
    !Number.isFinite(item.loadMs) ||
    item.loadMs < 0 ||
    ![item.cold, item.warm, item.longer].every((value) => timing(value, backend))
  )
    throw new Error('invalid_benchmark');
}
export function benchmarkSnapshot(value: unknown): BenchmarkSnapshot {
  const item = object(value, [
    'state',
    'stage',
    'devices',
    'activeDeviceId',
    'report',
    'failureCode',
  ]);
  if (
    !stringEnum(item.state, ['idle', 'running', 'done', 'failed', 'cancelled']) ||
    !stringEnum(item.stage, ['devices', ...stages]) ||
    typeof item.report !== 'string' ||
    item.report.length > 128000 ||
    !validFailure(item.failureCode, item.state) ||
    !Array.isArray(item.devices) ||
    item.devices.length > 64
  )
    throw new Error('invalid_benchmark');
  const ids = new Set<string>();
  let running = 0;
  for (const value of item.devices) {
    const device = object(value, [
      'id',
      'name',
      'backend',
      'state',
      'stage',
      'result',
      'failureCode',
      'failureDetail',
    ]);
    if (
      typeof device.id !== 'string' ||
      !/^(cpu|gpu-[1-9][0-9]*)$/.test(device.id) ||
      ids.has(device.id) ||
      typeof device.name !== 'string' ||
      device.name.length < 1 ||
      device.name.length > 160 ||
      /[\u0000-\u001f\u007f]/.test(device.name) ||
      !stringEnum(device.backend, ['cpu', 'metal', 'vulkan']) ||
      (device.id === 'cpu') !== (device.backend === 'cpu') ||
      !stringEnum(device.state, ['pending', 'running', 'done', 'failed', 'cancelled']) ||
      !stringEnum(device.stage, stages) ||
      !validFailure(device.failureCode, device.state) ||
      !(
        device.failureDetail === null ||
        (device.failureCode !== null && stringEnum(device.failureDetail, failureDetails))
      )
    )
      throw new Error('invalid_benchmark');
    ids.add(device.id);
    if (device.state === 'done') {
      if (device.stage !== 'long') throw new Error('invalid_benchmark');
      result(device.result, device.backend);
    } else if (device.result !== null) throw new Error('invalid_benchmark');
    if (device.state === 'running') {
      running++;
      if (item.state !== 'running' || item.activeDeviceId !== device.id)
        throw new Error('invalid_benchmark');
    }
    if (device.state === 'cancelled' && item.state !== 'cancelled')
      throw new Error('invalid_benchmark');
  }
  if (
    running > 1 ||
    (item.state !== 'running' && item.activeDeviceId !== null) ||
    (item.activeDeviceId !== null &&
      (typeof item.activeDeviceId !== 'string' || !ids.has(item.activeDeviceId))) ||
    (item.state === 'idle' && item.devices.length !== 0) ||
    (item.state === 'done' &&
      (item.devices.length === 0 || item.devices.some((device) => device.state !== 'done'))) ||
    (item.state === 'failed' &&
      item.devices.some((device) => ['running', 'pending', 'cancelled'].includes(device.state)))
  )
    throw new Error('invalid_benchmark');
  return item as unknown as BenchmarkSnapshot;
}
export function benchmarkSpeed(result: BenchmarkResult): number {
  return (
    (result.shortAudioMs + result.longAudioMs) /
    (result.warm.milliseconds + result.longer.milliseconds)
  );
}
export function backendLabel(backend: BenchmarkTiming['backend']): string {
  const labels = { cpu: 'CPU · Процессор', vulkan: 'GPU · Vulkan', metal: 'GPU · Metal' };
  return labels[backend as keyof typeof labels];
}
export function gpuLabel(platform: 'macos' | 'windows'): string {
  return backendLabel(platform === 'macos' ? 'metal' : 'vulkan');
}
