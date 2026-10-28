/**
 * AJ-E1542: the switch flow's capability check: what the target lacks,
 * what the user picked (safe items start ticked), and setting the picks up
 * just before the switch. The check never blocks a switch: if it fails,
 * there's simply no list.
 */
import { useEffect, useState } from "react";
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

  // "Check again" reads the target once more (after the user turned
  // something on in its account), keeping what they had ticked.
  const [round, setRound] = useState(0);
  const [checking, setChecking] = useState(false);
  useEffect(() => {
    let alive = true;
    // A new target starts clean: nothing picked or set up for the last one
    // carries over, even when this check fails (review of #132).
    if (round === 0) {
      setSetup(null);
      setPicked(new Set());
      setResults([]);
    }
    setChecking(true);
    switchSetupCheck(sessionId, targetProfileId)
      .then((check) => {
        if (!alive) return;
        setSetup(check);
        setPicked((current) =>
          round === 0
            ? new Set(check.offers.filter((offer) => offer.suggested).map((offer) => offer.id))
            : new Set(check.offers.filter((offer) => current.has(offer.id)).map((offer) => offer.id)),
        );
      })
      .catch(() => {
        if (alive && round === 0) setSetup(null);
      })
      .finally(() => {
        if (alive) setChecking(false);
      });
    return () => {
      alive = false;
    };
  }, [sessionId, targetProfileId, round]);
  useEffect(() => setRound(0), [sessionId, targetProfileId]);
  const recheck = () => setRound((n) => n + 1);

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

  /** Sets up what's picked; resolves to what was done (empty: nothing). */
  const applyPicked = async (): Promise<AppliedInfo[]> => {
    const applied = picked.size > 0 ? await switchSetupApply(sessionId, targetProfileId, [...picked]) : [];
    setResults(applied);
    return applied;
  };

  return { setup, picked, toggle, results, setResults, applyPicked, recheck, checking };
}
