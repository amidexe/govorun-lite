// Snapshot contracts contain JSON values. Property order is not observable;
// array order is observable. Equal replies keep the last published identity.
export function sameSnapshot(left: unknown, right: unknown): boolean {
  if (Object.is(left, right)) return true;
  if (left === null || right === null || typeof left !== 'object' || typeof right !== 'object')
    return false;
  if (Array.isArray(left) || Array.isArray(right))
    return Array.isArray(left) && Array.isArray(right) && left.length === right.length &&
      left.every((value, index) => sameSnapshot(value, right[index]));
  const a = left as Record<string, unknown>, b = right as Record<string, unknown>;
  const keys = Object.keys(a);
  return keys.length === Object.keys(b).length && keys.every(key =>
    Object.prototype.hasOwnProperty.call(b, key) && sameSnapshot(a[key], b[key]));
}

export function distinctSnapshots<T>(publish: (value: T) => void) {
  let initialized = false;
  let previous: T;
  return (value: T): boolean => {
    if (initialized && sameSnapshot(previous, value)) return false;
    previous = value;
    initialized = true;
    publish(value);
    return true;
  };
}
