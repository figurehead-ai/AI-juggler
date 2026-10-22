/**
 * First run: with zero accounts, the first-run flow opens by itself
 * (spec §12.1, D-25), once per dismissal. After Reopen AI Juggler
 * (Screen Recording, the 0.0.12 report) it comes back at its access
 * step, whatever else. Returns its open state and whether it resumes
 * at access.
 */
import { useEffect, useState } from "react";
import { accountsList } from "./bridge";
import { takeResumeAccess } from "./osAccess";

export function useFirstRun(): [boolean, (open: boolean) => void, boolean, (atAccess: boolean) => void] {
  const [open, setOpen] = useState(false);
  const [atAccess, setAtAccess] = useState(false);
  useEffect(() => {
    if (takeResumeAccess()) {
      setAtAccess(true);
      setOpen(true);
      return;
    }
    if (localStorage.getItem("aij.onboarding.dismissed") === "1") return;
    void accountsList().then((accounts) => {
      if (accounts.length === 0) setOpen(true);
    });
  }, []);
  return [open, setOpen, atAccess, setAtAccess];
}
