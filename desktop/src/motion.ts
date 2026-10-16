export function motionDuration() {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 180;
}
