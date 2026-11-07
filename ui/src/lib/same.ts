/**
 * 0.4.1, idle CPU: an answer equal to the one shown changes nothing, so it
 * doesn't become new state (a new array each poll re-rendered every
 * screen that reads it). Structural equality for the backend's answers:
 * plain objects, arrays, and primitives (what JSON carries).
 */
export function same(a: unknown, b: unknown): boolean {
  if (Object.is(a, b)) return true;
  if (typeof a !== "object" || typeof b !== "object" || a === null || b === null) return false;
  if (Array.isArray(a)) {
    if (!Array.isArray(b) || a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) if (!same(a[i], b[i])) return false;
    return true;
  }
  if (Array.isArray(b)) return false;
  if (a instanceof Set || b instanceof Set || a instanceof Map || b instanceof Map) return false;
  const ak = Object.keys(a);
  if (ak.length !== Object.keys(b).length) return false;
  for (const key of ak) {
    if (!Object.prototype.hasOwnProperty.call(b, key)) return false;
    if (!same(Reflect.get(a, key), Reflect.get(b, key))) return false;
  }
  return true;
}

/** The answer to keep: `was` itself when `next` equals it (its identity
 * kept, so nothing that reads it renders again), else `next`. */
export function keepIfSame<T>(was: T, next: T): T {
  return same(was, next) ? was : next;
}
