import type { RuntimeSnapshot } from '../features/dictation/runtime';
import type { DictionarySnapshot, DictionaryToggle } from './generated/dictionary';
import type { StatisticsSnapshot } from '../features/statistics/model.ts';
import type { AccountSnapshot } from '../features/account/model';
import type { AppearanceSnapshot, ThemeMode } from '../features/appearance/model';
import type { AboutSnapshot, AboutOpenLink } from './generated/about';
import type { BenchmarkSnapshot } from '../features/benchmark/model';
import type { BillingSnapshot } from '../features/billing/model';
import type { ClipboardSnapshot } from '../features/clipboard/model';
import type { DiagnosticsSnapshot } from '../features/diagnostics/model';
import type { DictationSnapshot } from '../features/dictation/model';
import type { HotkeySnapshot } from '../features/hotkeys/model';
import type { RecordingDiagnosticState } from '../features/recording-diagnostics/model';
import type { SelfHostSnapshot } from '../features/self-host/model';
import type { SetupSnapshot, SetupProgress } from '../features/setup/model';
import type { StylesSnapshot } from '../features/styles/model';
import type { UpdateSnapshot, UpdateChannel } from '../features/updates/model';
import type {
  ActionPayload,
  AccountActions,
  BillingActions,
  DictationAction,
  SelfHostActions,
  StylesActions,
} from './actions';

type Command<Params, Result> = { params: Params; result: Result };
type Scoped<Actions> = ActionPayload<Actions> & { scope: string };
type HotkeyAction =
  | { action: 'start' | 'reset'; target: 'dictation' | 'plain' | 'cancel' }
  | { action: 'cancel'; captureID: string };

/** Renderer-facing commands only. Host-private commands never enter this port. */
export interface DesktopCommands {
  'window.layout': Command<{ home: boolean }, void>;
  'runtime.snapshot': Command<undefined, RuntimeSnapshot>;
  'runtime.reconnect': Command<undefined, RuntimeSnapshot>;
  'runtime.shutdown': Command<undefined, void>;
  'recording.cancel': Command<undefined, void>;
  'account.snapshot': Command<undefined, AccountSnapshot>;
  'account.action': Command<Scoped<AccountActions>, AccountSnapshot>;
  'billing.snapshot': Command<undefined, BillingSnapshot>;
  'billing.action': Command<Scoped<BillingActions>, BillingSnapshot>;
  'styles.snapshot': Command<undefined, StylesSnapshot>;
  'styles.action': Command<Scoped<StylesActions>, StylesSnapshot>;
  'selfHost.snapshot': Command<undefined, SelfHostSnapshot>;
  'selfHost.action': Command<
    Scoped<SelfHostActions> | { action: 'cancel'; operationID: string },
    SelfHostSnapshot
  >;
  'dictation.snapshot': Command<undefined, DictationSnapshot>;
  'dictation.action': Command<DictationAction, DictationSnapshot>;
  'hotkeys.snapshot': Command<undefined, HotkeySnapshot>;
  'hotkeys.action': Command<HotkeyAction, HotkeySnapshot>;
  'dictionary.snapshot': Command<undefined, DictionarySnapshot>;
  'dictionary.replace': Command<DictionarySnapshot, DictionarySnapshot>;
  'dictionary.setEnabled': Command<DictionaryToggle, DictionarySnapshot>;
  'statistics.snapshot': Command<undefined, StatisticsSnapshot>;
  'statistics.reset': Command<{ confirmed: true }, StatisticsSnapshot>;
  'clipboard.snapshot': Command<undefined, ClipboardSnapshot>;
  'clipboard.action': Command<ClipboardSnapshot, ClipboardSnapshot>;
  'diagnostics.snapshot': Command<undefined, DiagnosticsSnapshot>;
  'diagnostics.action': Command<
    | { action: 'clear'; confirmed: true }
    | { action: 'copy' | 'copyReport' | 'export' }
    | { action: 'extended'; enabled: boolean },
    DiagnosticsSnapshot | null
  >;
  'recordingDiagnostics.snapshot': Command<undefined, RecordingDiagnosticState>;
  'recordingDiagnostics.action': Command<
    { action: 'start'; confirmed: true } | { action: 'stop' | 'clear' | 'open' },
    RecordingDiagnosticState
  >;
  'benchmark.snapshot': Command<undefined, BenchmarkSnapshot>;
  'benchmark.action': Command<{ action: 'start' | 'cancel' | 'copy' }, BenchmarkSnapshot>;
  'about.snapshot': Command<undefined, AboutSnapshot>;
  'about.openLink': Command<AboutOpenLink, void>;
  'updates.snapshot': Command<undefined, UpdateSnapshot>;
  'updates.check': Command<{ manual: boolean }, UpdateSnapshot>;
  'updates.channel': Command<{ channel: UpdateChannel }, UpdateSnapshot>;
  'updates.dismiss': Command<{ version: string }, UpdateSnapshot>;
  'updates.download': Command<{ version: string }, UpdateSnapshot>;
  'updates.install': Command<{ version: string }, UpdateSnapshot>;
  'updates.cancel': Command<undefined, UpdateSnapshot>;
  'appearance.snapshot': Command<undefined, AppearanceSnapshot>;
  'appearance.set': Command<{ mode: ThemeMode }, AppearanceSnapshot>;
  'setup.snapshot': Command<undefined, SetupSnapshot>;
  'setup.save': Command<{ expectedRevision: number; progress: SetupProgress }, SetupSnapshot>;
  'setup.reset': Command<{ confirmed: true }, SetupSnapshot>;
}

export type DesktopCommand = keyof DesktopCommands;
export type CommandResult<Method extends DesktopCommand> = DesktopCommands[Method]['result'];
export type CommandArguments<Method extends DesktopCommand> =
  DesktopCommands[Method]['params'] extends undefined
    ? []
    : [params: DesktopCommands[Method]['params']];
export type SnapshotCommand = {
  [Method in DesktopCommand]: Method extends `${string}.snapshot` ? Method : never;
}[DesktopCommand];

export interface DesktopInvoker {
  invoke<Method extends DesktopCommand>(
    method: Method,
    ...args: CommandArguments<Method>
  ): Promise<CommandResult<Method>>;
}
