/**
 * A module store's listeners, for `useSyncExternalStore`: `subscribe`
 * hands a view's listener in (and back out on unmount), `notify` tells
 * every view the store changed.
 */
export interface Listeners {
  subscribe: (listener: () => void) => () => void;
  notify: () => void;
}

export function listeners(): Listeners {
  const set = new Set<() => void>();
  return {
    subscribe: (listener) => {
      set.add(listener);
      return () => set.delete(listener);
    },
    notify: () => {
      for (const listener of set) listener();
    },
  };
}
