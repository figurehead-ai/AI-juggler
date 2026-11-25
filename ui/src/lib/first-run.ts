/**
 * First run: the setup wizard's open state. A new install (no accounts)
 * opens on the Welcome (AJ-E1545 R1: "Let's get you set up", Get
 * started, Bot's lead card); its Get started opens the wizard, never the
 * app by itself (the 0.5.1 PL1 walk's F20: an empty data folder opened
 * straight on step 1, so nobody saw the Welcome). After Reopen AI
 * Juggler (Screen Recording, the 0.0.12 report) it comes back at its
 * access step, whatever else. Returns its open state and whether it
 * resumes at access.
 */
import { useEffect, useState } from "react";
import { takeResumeAccess } from "./osAccess";

export function useFirstRun(): [boolean, (open: boolean) => void, boolean, (atAccess: boolean) => void] {
  const [open, setOpen] = useState(false);
  const [atAccess, setAtAccess] = useState(false);
  useEffect(() => {
    if (takeResumeAccess()) {
      setAtAccess(true);
      setOpen(true);
    }
  }, []);
  return [open, setOpen, atAccess, setAtAccess];
}
