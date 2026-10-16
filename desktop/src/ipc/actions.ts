import type { ApplicationRule } from '../features/styles/application-rules';
import type { TextStyleOptions } from '../features/styles/model';
import type { SelfHostConfiguration } from '../features/self-host/model';
import type { PaymentSource } from '../features/billing/payment-source';

export type ActionPayload<Actions> = {
  [Action in keyof Actions]: { action: Action } & Actions[Action];
}[keyof Actions];

export type ActionArguments<Actions> = {
  [Action in keyof Actions]: keyof Actions[Action] extends never
    ? [action: Action]
    : [action: Action, params: Actions[Action]];
}[keyof Actions];

/** Preserves the action/arguments relationship when constructing a wire request. */
export function actionPayload<Actions>(args: ActionArguments<Actions>): ActionPayload<Actions> {
  const [action, params] = args;
  return { ...params, action } as ActionPayload<Actions>;
}

export interface AccountActions {
  signIn: { provider: 'yandex' | 'vk' };
  cancelSignIn: {};
  signOut: {};
  refresh: {};
  merge: {};
  keepLocal: {};
  requestSync: {};
  recoverStorage: {};
  restartStorage: { confirmed: true };
  revokeDevice: { deviceId: string; confirmed: true };
  replaceDevice: { deviceId: string; confirmed: true };
  deleteAccount: { confirmed: true };
  openLink: { link: 'agreement' | 'privacy' };
}

export interface StylesActions {
  select: { selection: string };
  disable: {};
  acceptTerms: { termsVersion: number; selection: string; confirmed: true };
  cancel: {};
  refresh: {};
  documents: {};
  savePreset: { id: string | null; name: string; instruction: string; options: TextStyleOptions };
  deletePreset: { id: string; confirmed: true };
  option: {
    [Name in keyof TextStyleOptions]: { name: Name; value: TextStyleOptions[Name] };
  }[keyof TextStyleOptions];
  loadApplications: {};
  saveApplicationRule: Omit<ApplicationRule, 'name'> & { customOptions: boolean };
  deleteApplicationRule: { id: string; confirmed: true };
}

export interface BillingActions {
  refresh: {};
  resumeCheckout: {};
  restartBinding: {};
  previewCheckout: { offerId: string };
  choosePaymentMethod: { previewId: string; method: PaymentSource };
  confirmCheckout: { previewId: string; email: string | null; confirmed: true };
  dismissPreview: {};
  previewPromo: { code: string; offerId: string | null };
  confirmPromo: { previewId: string; confirmed: true };
  dismissPromo: {};
  dismissPromoAccess: { continuationId: string; deferred: boolean };
  refreshPromoAccess: { continuationId: string };
  activateStarter: {
    termsVersion: number;
    confirmed: true;
    programVersion?: string;
    selection: string;
    activationOptions?: TextStyleOptions;
  };
  cancelRenewal: { confirmed: true };
  cancelPlanChange: { confirmed: true };
  removeCard: { confirmed: true };
  support: {};
  legal: { previewId?: string; document: string };
}

export interface SelfHostActions {
  provider: { provider: SelfHostConfiguration['provider'] };
  connect: { configuration: SelfHostConfiguration; key: string; confirmed: true };
  disconnect: {};
  revokeConsent: {};
  removeKey: { confirmed: true };
  clear: { confirmed: true };
}

export interface DictationFields {
  microphoneId: string | null;
  playSounds: boolean;
  muteAudio: boolean;
  maxMinutes: number;
  acceleration: 'automatic' | 'cpu' | 'gpu';
  voiceMarker: 'off' | 'microphone' | 'text';
  launchAtLogin: boolean;
  pushToTalk: boolean;
  keepModelLoaded: boolean;
  alwaysOnMicrophone: boolean;
  inputChannel: number | null;
}

export type DictationSetting = {
  [Field in keyof DictationFields]: { action: 'set'; field: Field; value: DictationFields[Field] };
}[keyof DictationFields];

export type DictationAction =
  | DictationSetting
  | { action: 'prepare' | 'loginItems' }
  | { action: 'permission'; permission: 'microphone' | 'accessibility' };
