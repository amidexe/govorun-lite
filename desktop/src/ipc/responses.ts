import { runtimeSnapshot } from '../features/dictation/runtime.ts';
import { dictionarySnapshot } from '../features/dictionary/model.ts';
import { statisticsSnapshot } from '../features/statistics/model.ts';
import { accountHostSnapshot } from '../features/account/host-snapshot.ts';
import { appearanceSnapshot } from '../features/appearance/model.ts';
import { aboutSnapshot } from '../features/about/model.ts';
import { benchmarkSnapshot } from '../features/benchmark/model.ts';
import { billingSnapshot } from '../features/billing/model.ts';
import { clipboardSnapshot } from '../features/clipboard/model.ts';
import { diagnosticsSnapshot } from '../features/diagnostics/model.ts';
import { dictationSnapshot } from '../features/dictation/model.ts';
import { hotkeySnapshot } from '../features/hotkeys/model.ts';
import { recordingDiagnosticState } from '../features/recording-diagnostics/model.ts';
import { selfHostSnapshot } from '../features/self-host/model.ts';
import { setupSnapshot } from '../features/setup/model.ts';
import { stylesSnapshot } from '../features/styles/model.ts';
import { updateSnapshot } from '../features/updates/model.ts';
import type { CommandResult, DesktopCommand } from './commands.ts';

const completed = (_value: unknown): void => {};
function diagnosticsResponse(value: unknown, request?: object) {
  if (
    request &&
    'action' in request &&
    ['copy', 'copyReport', 'export'].includes(String(request.action)) &&
    value === null
  )
    return null;
  return diagnosticsSnapshot(value);
}

/** Untrusted transport responses become application values at this boundary. */
export const responses: {
  [Method in DesktopCommand]: (value: unknown, request?: object) => CommandResult<Method>;
} = {
  'window.layout': completed,
  'runtime.snapshot': runtimeSnapshot,
  'runtime.reconnect': runtimeSnapshot,
  'runtime.shutdown': completed,
  'recording.cancel': completed,
  'account.snapshot': accountHostSnapshot,
  'account.action': accountHostSnapshot,
  'billing.snapshot': billingSnapshot,
  'billing.action': billingSnapshot,
  'styles.snapshot': stylesSnapshot,
  'styles.action': stylesSnapshot,
  'selfHost.snapshot': selfHostSnapshot,
  'selfHost.action': selfHostSnapshot,
  'dictation.snapshot': dictationSnapshot,
  'dictation.action': dictationSnapshot,
  'hotkeys.snapshot': hotkeySnapshot,
  'hotkeys.action': hotkeySnapshot,
  'dictionary.snapshot': dictionarySnapshot,
  'dictionary.replace': dictionarySnapshot,
  'dictionary.setEnabled': dictionarySnapshot,
  'statistics.snapshot': statisticsSnapshot,
  'statistics.reset': statisticsSnapshot,
  'clipboard.snapshot': clipboardSnapshot,
  'clipboard.action': clipboardSnapshot,
  'diagnostics.snapshot': diagnosticsSnapshot,
  'diagnostics.action': diagnosticsResponse,
  'recordingDiagnostics.snapshot': recordingDiagnosticState,
  'recordingDiagnostics.action': recordingDiagnosticState,
  'benchmark.snapshot': benchmarkSnapshot,
  'benchmark.action': benchmarkSnapshot,
  'about.snapshot': aboutSnapshot,
  'about.openLink': completed,
  'updates.snapshot': updateSnapshot,
  'updates.check': updateSnapshot,
  'updates.channel': updateSnapshot,
  'updates.dismiss': updateSnapshot,
  'updates.download': updateSnapshot,
  'updates.install': updateSnapshot,
  'updates.cancel': updateSnapshot,
  'appearance.snapshot': appearanceSnapshot,
  'appearance.set': appearanceSnapshot,
  'setup.snapshot': setupSnapshot,
  'setup.save': setupSnapshot,
  'setup.reset': setupSnapshot,
};
