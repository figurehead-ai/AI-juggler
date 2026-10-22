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

  useEffect(() => {
    let alive = true;
    setSetup(null);
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

  return { setup, picked, toggle, results, setResults, applyPicked };
}
