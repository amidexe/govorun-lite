// @generated from shared/core serde JSON Schema; do not edit.
export interface ApplicationChoice {
  id: string;
  name: string;
}
export interface ApplicationRule {
  emojiMode: string | null;
  id: string;
  name: string;
  reducedPunctuation: boolean | null;
  style: string | null;
}
export interface StyleChoice {
  detail: string;
  id: string;
  options: TextStyleOptions;
  title: string;
}
export interface StylePreset {
  id: string;
  instruction: string;
  name: string;
  options: TextStyleOptions;
}
export interface StylesProcessingLimits {
  maximumCharacters: number;
  maximumProcessingSeconds: number;
  minimumCharacters: number;
  stopTailMilliseconds: number;
}
export interface StylesSnapshot {
  applicationRules: Array<ApplicationRule>;
  applications: Array<ApplicationChoice>;
  availability: string | null;
  availabilityReason: string;
  busy: boolean;
  canCreatePreset: boolean;
  displayState: string;
  enabled: boolean;
  gate: string;
  limitReason: string;
  maxPresets: number;
  message: string | null;
  onlineServicePaused: boolean;
  options: TextStyleOptions;
  presets: Array<StylePreset>;
  processingLimits: StylesProcessingLimits;
  processingReadiness: string;
  resumeChoice: boolean;
  scope: string;
  selected: string | null;
  styles: Array<StyleChoice>;
  termsAccepted: boolean;
  termsVersion: number;
  title: string;
}
export interface TextStyleOptions {
  emojiMode: string;
  profanityFilter: boolean;
  reducedPunctuation: boolean;
}
