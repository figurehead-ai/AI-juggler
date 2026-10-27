/**
 * AJ-E1542: the switch flow's capability check: what the target lacks,
 * what the user picked (safe items start ticked), and setting the picks up
 * just before the switch. The check never blocks a switch: if it fails,
 * there's simply no list.
 */
import { useCallback, useEffect, useState } from "react";
import {
  switchSetupApply,
  switchSetupCheck,
  type AppliedInfo,
  type SetupCheckInfo,
} from "./ipc/switchSetup";

export function useSwitchSetup(sessionId: string, targetProfileId: string) {
  const [setup, setSetup] = useState<SetupCheckInfo | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [results, setResults] = useState<AppliedInfo[]>([]);

  useEffect(() => {
    let alive = true;
    // A new target starts clean: nothing picked or set up for the last one
    // carries over, even when this check fails (review of #132).
    setSetup(null);
    setPicked(new Set());
    setResults([]);
    switchSetupCheck(sessionId, targetProfileId)
      .then((check) => {
        if (!alive) return;
        setSetup(check);
        setPicked(new Set(check.offers.filter((offer) => offer.suggested).map((offer) => offer.id)));
      })
      .catch(() => {
        if (alive) setSetup(null);
      });
    return () => {
      alive = false;
    };
  }, [sessionId, targetProfileId]);

  /** Check again: reads both accounts again (after the user turned
   * something on), keeping the ticks of what's still offered; resolves to
   * what it found, in words. */
  const recheck = useCallback(async (): Promise<string> => {
    // What this conversation used (or nothing says) and the account turns on.
    const toTurnOn = (offers: SetupCheckInfo["offers"]) =>
      offers.filter((offer) => offer.verdict === "turn-on" && offer.used !== false).length;
    const before = toTurnOn(setup?.offers ?? []);
    const check = await switchSetupCheck(sessionId, targetProfileId);
    setSetup(check);
    setPicked((current) => new Set([...current].filter((id) => check.offers.some((offer) => offer.id === id))));
    const after = toTurnOn(check.offers);
    if (after < before) return `${check.to} has ${before - after} of them now.`;
    const them = after === 1 ? "it" : "them";
    return `${check.to} doesn't have ${them} yet. Each arrives with ${check.to}'s next session after you turn it on.`;
  }, [sessionId, targetProfileId, setup]);

  const toggle = (id: string) => {
    setPicked((current) => {
      const next = new Set(current);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  /** Sets up what's picked; resolves to what was done (empty: nothing).
   * With something to turn on, the backend is asked even with nothing
   * picked: it keeps the reminder for Settings. */
  const applyPicked = async (): Promise<AppliedInfo[]> => {
    const toTurnOn = setup?.offers.some((offer) => offer.verdict === "turn-on" && offer.used !== false) ?? false;
    const applied =
      picked.size > 0 || toTurnOn ? await switchSetupApply(sessionId, targetProfileId, [...picked]) : [];
    setResults(applied);
    return applied;
  };

  return { setup, picked, toggle, recheck, results, setResults, applyPicked };
}
