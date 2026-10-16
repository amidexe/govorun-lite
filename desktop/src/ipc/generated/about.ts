// @generated from shared/core serde JSON Schema; do not edit.
export interface AboutOpenLink {
  id: string;
}
export interface AboutSnapshot {
  buildID: string;
  keyStorage: string;
  licenses: Array<ApplicationLicense>;
  nativeBuildID: string;
  nativeVersion: string;
  platformBuild: string;
  version: string;
}
export interface ApplicationLicense {
  copyright: string;
  id: string;
  name: string;
  spdx: string;
}
export interface NativeBuildFacts {
  buildID: string;
  platformBuild: string;
  version: string;
}
