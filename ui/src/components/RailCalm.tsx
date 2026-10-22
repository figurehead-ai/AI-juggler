import { useContext, type JSX } from "react";
import { ThisMacContext, machineStatus } from "../lib/machine";

/** The collapsed rail's "All clear": nothing running, and only while the
 * Mac itself is calm — the same state This Mac and the footer read
 * (QA, P19: it said "All clear" under memory pressure). */
export function RailCalm(): JSX.Element | null {
  const status = machineStatus(useContext(ThisMacContext));
  if (status && !status.calm) return null;
  return (
    <span className="px-1 py-2 text-center text-[9px] leading-relaxed text-faint" data-testid="rail-all-clear">
      All clear
    </span>
  );
}
