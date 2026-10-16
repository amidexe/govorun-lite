// @generated from shared/core serde JSON Schema; do not edit.
export interface SelfHostConfiguration {
  endpoint: string;
  model: string;
  prompt: string;
  provider: SelfHostProvider;
  proxyEnabled: boolean;
  proxyURL: string;
}
export type SelfHostKeyState = "missing" | "saved" | "inaccessible" | "staleReference";
export type SelfHostProvider = "ollama" | "openai" | "gemini";
export interface SelfHostSnapshot {
  busy: boolean;
  cancelling: boolean;
  configuration: SelfHostConfiguration;
  configured: boolean;
  consent: boolean;
  failed: boolean;
  keyState: SelfHostKeyState;
  message: string | null;
  operation: string | null;
  operationID: string | null;
  routeEnabled: boolean;
  scope: string;
  stylingEnabled: boolean;
  verified: boolean;
}
