/**
 * First run: with zero accounts, the first-run flow opens by itself
 * (spec §12.1, D-25), once per dismissal. Returns its open state.
 */
import { useEffect, useState } from "react";
import { accountsList } from "./bridge";

export function useFirstRun(): [boolean, (open: boolean) => void] {
  const [open, setOpen] = useState(false);
  useEffect(() => {
    if (localStorage.getItem("aij.onboarding.dismissed") === "1") return;
    void accountsList().then((accounts) => {
      if (accounts.length === 0) setOpen(true);
    });
  }, []);
  return [open, setOpen];
}
