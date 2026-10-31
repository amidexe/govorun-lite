// @generated from shared/core serde JSON Schema; do not edit.
export type DictationAcceleration = "automatic" | "cpu" | "gpu";
export type DictationDuration = "local" | "styles" | "unlimited";
export type DictationLogin = "enabled" | "disabled" | "approvalRequired" | "unavailable";
export type DictationMarker = "off" | "microphone" | "text";
export type DictationMicrophone = "authorized" | "notDetermined" | "denied" | "restricted" | "unknown";
export interface DictationMicrophoneDevice {
  id: string;
  name: string;
}
export type DictationPause = "auto" | "short" | "medium" | "long";
export interface DictationPermissions {
  accessibility: boolean;
  microphone: DictationMicrophone;
}
export interface DictationSnapshot {
  acceleration: DictationAcceleration;
  alwaysOnMicrophone?: boolean;
  durationMode: DictationDuration;
  inputChannel?: number | null;
  inputChannels?: number;
  launchAtLogin: DictationLogin;
  markerMinimum: number;
  maxMinutes: number;
  microphoneId: string | null;
  microphones: Array<DictationMicrophoneDevice>;
  modelUnloadMinutes: number;
  muteAudio: boolean;
  pause: DictationPause;
  permissions: DictationPermissions;
  playSounds: boolean;
  pushToTalk?: boolean;
  voiceMarker: DictationMarker;
}
