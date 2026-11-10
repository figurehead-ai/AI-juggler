/**
 * First run's look for setups: the scan, and what the found step says.
 * A scan that failed says so and can be run again; it's never shown as
 * "none found" (that would send the user to add an account they have).
 */
import { useCallback, useRef, useState } from "react";
import { environmentScanFull, type ScanReportData } from "../lib/ipc/providers";
import { errorText } from "../lib/bridge";
import { funnelEvent } from "../lib/funnel";
import { ownSignIn } from "../lib/providers";
import { listWords } from "../lib/account-terms";

const NOTHING = { installs: [], probes: [], seeds: [], desktop_apps: [], cancelled: false } as unknown as ScanReportData;

/**
 * The scan's report (`null` while it runs), why it failed, and a rescan.
 * Only the newest scan's answer lands: a Look again pressed while an
 * earlier scan still runs must not be overtaken by that earlier, older
 * answer when it finishes first.
 */
export function useSetupScan(): { report: ScanReportData | null; scanError: string | null; scan: () => void } {
  const [report, setReport] = useState<ScanReportData | null>(null);
  const [scanError, setScanError] = useState<string | null>(null);
  const latest = useRef(0);
  const scan = useCallback(() => {
    const run = ++latest.current;
    setReport(null);
    setScanError(null);
    funnelEvent("scan_started");
    void environmentScanFull().then(
      (found) => {
        if (run !== latest.current) return;
        setReport(found);
        funnelEvent("scan_completed");
      },
      (error) => {
        if (run !== latest.current) return;
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
  /** AJ-E3306: the tools installed with nothing set up (their names),
   * offered as fresh apps below the setups. */
  notSetUp: string[] = [],
): { heading: string; lead: string } {
  if (scanError !== null) {
    const why = scanError.trim();
    return {
      heading: "AI Juggler couldn't look for your setups.",
      lead: `${/[.!?…]$/.test(why) ? why : `${why}.`} Try again, or add an app or import a folder yourself.`,
    };
  }
  if (count === 0 && notSetUp.length > 0) {
    return {
      heading: `${listWords(notSetUp)} ${notSetUp.length === 1 ? "is" : "are"} installed, with nothing set up yet.`,
      lead: "Pick the ones to set up. AI Juggler sets each up in its own folder, and you sign in to each after.",
    };
  }
  if (count === 0) {
    return {
      heading: `No ${toolName} setups found yet.`,
      lead: `Add your first app and AI Juggler sets it up in its own folder. You'll sign in with ${ownSignIn(provider)}.`,
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

/** The error, without the internal "account directory error at <path>:"
 * prefix — the row already says which setup it is. */
export function plainImportError(text: string): string {
  const plain = text.replace(/^account directory error at .+?: (?=\S)/, "");
  return plain.charAt(0).toUpperCase() + plain.slice(1);
}

/** The result line: what came in, what didn't, what already was. */
export function outcomeWords({
  stopped,
  done,
  waiting,
  failed,
  already,
}: {
  stopped: boolean;
  done: number;
  waiting: number;
  failed: number;
  already: string[];
}): string {
  const parts: string[] = [];
  if (stopped) parts.push(`Stopped. ${done} brought in${waiting > 0 ? `, ${waiting} still to go` : ""}.`);
  else if (done > 0 || failed === 0) parts.push(done === 1 ? "1 brought in." : `${done} brought in.`);
  if (failed > 0) parts.push(failed === 1 ? "1 couldn't come in." : `${failed} couldn't come in.`);
  if (already.length === 1) parts.push(`${already[0]} was already in AI Juggler.`);
  else if (already.length > 1) parts.push(`${already.length} were already in AI Juggler.`);
  if (done === 0 && failed === 0 && !stopped && already.length > 0) return parts.slice(1).join(" ");
  return parts.join(" ");
}
