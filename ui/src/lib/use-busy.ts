/**
 * One action at a time on a control (a pool's seal, an App's connector):
 * `busy` while it runs, so its buttons wait; why it failed, said in place,
 * and cleared when the next one starts.
 */
import { useState } from "react";
import { errorText } from "./bridge";

export function useBusy(): { busy: boolean; error: string | null; act: (work: () => Promise<void>) => Promise<void> } {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const act = async (work: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await work();
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };
  return { busy, error, act };
}
