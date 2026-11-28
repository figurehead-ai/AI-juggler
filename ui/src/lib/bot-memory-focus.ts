/**
 * AJ-E3832 G9: the memory a chip asked What Bot remembers to show. Set
 * just before the tab opens, read once when the list mounts.
 */
let pending: string | null = null;

/** Asks the memories list to show this memory (its key). */
export function focusMemory(key: string | null): void {
  pending = key;
}

/** The memory to show, once. */
export function takeMemoryFocus(): string | null {
  const key = pending;
  pending = null;
  return key;
}
