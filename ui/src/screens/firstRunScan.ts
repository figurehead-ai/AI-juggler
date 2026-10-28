/**
 * First run's look for setups: the scan, and what the found step says.
 * A scan that failed says so and can be run again; it's never shown as
 * "none found" (that would send the user to add an account they have).
 */
import { useCallback, useState } from "react";
import { environmentScanFull, errorText, type ScanReportData } from "../lib/bridge";
import { funnelEvent } from "../lib/funnel";
import { ownSignIn } from "../lib/providers";

const NOTHING = { installs: [], probes: [], seeds: [], desktop_apps: [], cancelled: false } as unknown as ScanReportData;

/** The scan's report (`null` while it runs), why it failed, and a rescan. */
export function useSetupScan(): { report: ScanReportData | null; scanError: string | null; scan: () => void } {
  const [report, setReport] = useState<ScanReportData | null>(null);
  const [scanError, setScanError] = useState<string | null>(null);
  const scan = useCallback(() => {
    setReport(null);
    setScanError(null);
    funnelEvent("scan_started");
    void environmentScanFull().then(
      (found) => {
        setReport(found);
        funnelEvent("scan_completed");
      },
      (error) => {
        setScanError(errorText(error));
        setReport(NOTHING);
      },
    );
  }, []);
  return { report, scanError, scan };
}

/** The found step's heading and the line under it. */
export function foundWords(
  scanError: string | null,
  count: number,
  toolName: string,
  kind: string,
  inPlace: boolean,
  provider: string,
): { heading: string; lead: string } {
  if (scanError !== null) {
    const why = scanError.trim();
    return {
      heading: "AI Juggler couldn't look for your setups.",
      lead: `${/[.!?…]$/.test(why) ? why : `${why}.`} Try again, or add an account or import a folder yourself.`,
    };
  }
  if (count === 0) {
    return {
      heading: `No ${toolName} setups found yet.`,
      lead: `Add your first account and AI Juggler sets it up in its own folder. You'll sign in with ${ownSignIn(provider)}.`,
    };
  }
  if (count === 1) {
    return {
      heading: `You already have a ${kind} setup on this computer.`,
      lead: inPlace
        ? "AI Juggler uses it where it is, so it stays signed in and keeps its history, the same as everywhere else you use it."
        : "AI Juggler makes its own copy; your original is never changed, and you can keep using it as before.",
    };
  }
  return {
    heading: `You already have ${count} ${kind ? `${kind} setups` : "setups"} on this computer.`,
    lead: inPlace
      ? "Pick the ones to bring in. AI Juggler uses each one where it is, so it stays signed in and keeps its history, the same as everywhere else you use it."
      : "Pick the ones to bring in. AI Juggler makes its own copy of each; your originals are never changed, and you can keep using them as before.",
  };
}
