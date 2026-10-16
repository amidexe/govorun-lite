import type {
  CommandArguments,
  CommandResult,
  DesktopCommand,
  DesktopInvoker,
} from './commands.ts';
import { responses } from './responses.ts';

export type DesktopTransport = (method: DesktopCommand, params: object) => Promise<unknown>;

export function desktopClient(transport: DesktopTransport): DesktopInvoker {
  return {
    async invoke<Method extends DesktopCommand>(
      method: Method,
      ...args: CommandArguments<Method>
    ): Promise<CommandResult<Method>> {
      if (!Object.hasOwn(responses, method)) throw new Error('unsupported_command');
      const value = await transport(method, args[0] ?? {});
      return responses[method](value, args[0]);
    },
  };
}
