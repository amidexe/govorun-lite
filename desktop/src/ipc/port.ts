export interface NativeEvent {
  event: string;
  data: unknown;
}
import type { DesktopInvoker } from './commands';
export interface DesktopApi extends DesktopInvoker {
  subscribe(listener: (event: NativeEvent) => void): () => void;
  window(
    action: 'close' | 'minimize' | 'settings' | 'home',
    route?:
      | 'general'
      | 'subscription'
      | 'account'
      | 'styles'
      | 'llm'
      | 'stats'
      | 'dictionary'
      | 'diagnostics'
      | 'about',
  ): void;
}

declare global {
  interface Window {
    govorun: DesktopApi;
  }
}
