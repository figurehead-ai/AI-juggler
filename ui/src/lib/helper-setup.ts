/**
 * AJ-E1547 — whether Bot's AI helper has been set up (its `helper`
 * system-prompt entry primed). Until then no helper starts, so the system
 * asks nothing; Bot's setup guide offers "Check this Mac", which shows
 * the gate's sheet first and then checks (`models-actions` checkAgain).
 * `null` while unknown; `true` where no helper can run (nothing to set up).
 */
import { useEffect, useState } from "react";
import { BOT_HELPER, osPromptStatus } from "./ipc/os-prompts";

export function useHelperSetUp(): boolean | null {
  const [setUp, setSetUp] = useState<boolean | null>(null);
  useEffect(() => {
    let live = true;
    void osPromptStatus(BOT_HELPER).then(
      (helper) => live && setSetUp(!helper || !helper.needed || helper.primed),
      () => live && setSetUp(true),
    );
    return () => {
      live = false;
    };
  }, []);
  return setUp;
}
