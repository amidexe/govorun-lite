// Browser-safe command names. Agent-only capabilities are never renderer commands.
import rendererCommands from '../../../shared/core/contracts/renderer-commands.json' with { type: 'json' };
export type DesktopMethod = keyof typeof rendererCommands;
export const desktopMethods = Object.keys(rendererCommands) as DesktopMethod[];
