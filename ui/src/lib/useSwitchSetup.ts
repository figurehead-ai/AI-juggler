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
  type OfferInfo,
  type SetupCheckInfo,
} from "./ipc/switchSetup";

/** D-51: the verdict of this project's notes when the other tool keeps
 * none: quoted in the handover by the switch itself, when ticked. */
export const IN_HANDOVER = "handover";

/** Something only the user can finish: what the other account turns on
 * itself, or a step of theirs (a server's secret, a sign-in). */
export function finishedByYou(offer: OfferInfo): boolean {
  return offer.verdict === "turn-on" || offer.verdict === "needs-you";
}

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
   * something on, or gave a server its secret), keeping the ticks of
   * what's still offered; resolves to what it found for `offerId`, in
   * words. */
  const recheck = useCallback(
    async (offerId: string): Promise<string> => {
      const check = await switchSetupCheck(sessionId, targetProfileId);
      setSetup(check);
      setPicked((current) => new Set([...current].filter((id) => check.offers.some((offer) => offer.id === id))));
      const still = check.offers.find((offer) => offer.id === offerId);
      if (!still) {
        const name = setup?.offers.find((offer) => offer.id === offerId)?.name;
        return `${check.to} has ${name ?? "it"} now.`;
      }
      const turnOn = still.verdict === "turn-on" ? ` It arrives with ${check.to}'s next session after you turn it on.` : "";
      return `${check.to} doesn't have it yet.${turnOn}`;
    },
    [sessionId, targetProfileId, setup],
  );

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

  /** D-51: the ticked offers the switch itself carries out (this
   * project's notes quoted in the handover), passed with the switch. */
  const approved = (): string[] => (setup?.offers ?? []).filter((offer) => offer.verdict === IN_HANDOVER && picked.has(offer.id)).map((offer) => offer.id);

  return { setup, picked, toggle, recheck, results, setResults, applyPicked, approved };
}
