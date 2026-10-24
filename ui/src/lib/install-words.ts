/**
 * A tool's installed version and what AI Juggler was tested with, in
 * words (the tool's page; moved out of RightInspector for its size).
 */
import type { ProviderInstallData } from "./bridge";
import { FOUND_UNCHECKED, versionUnchecked } from "./providers";

/** The installed version and verified range, in words. */
export function installWords(install: ProviderInstallData): string {
  const verified = install.verified_against ?? [];
  switch (install.status) {
    case "installed":
      return `${install.version ? `Version ${install.version}` : "Installed"}${verified.length > 0 ? ` · tested with ${verified.join(", ")}` : ""}`;
    case "unverified":
      // A tool no version of which was tried yet (E16's, until their
      // live check) isn't "newer" than anything.
      return verified.length > 0
        ? `Version ${install.version ?? "unknown"} · newer than AI Juggler has been tested with`
        : `Version ${install.version ?? "unknown"} · not tested with AI Juggler yet`;
    case "not_installed":
      return "Not installed on this Mac";
    default:
      if (versionUnchecked(install)) return FOUND_UNCHECKED;
      return "Couldn't check what's installed";
  }
}
