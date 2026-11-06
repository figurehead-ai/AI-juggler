/**
 * Finding and importing what is already on the Mac (AJ-E3xx): the
 * environment scan, import and inspecting one source.
 */
import { invoke, liveBackend as hasBackend } from "./transport";
import { DEFAULT_SCAN_REPORT } from "./providers";
import type { DiscoveredInfo } from "../../types/generated/DiscoveredInfo";
import type { ImportResult } from "../../types/generated/ImportResult";
import type { SourceInspectInfo } from "../../types/generated/SourceInspectInfo";

/**
 * Scans the machine for existing harness setups the onboarding wizard can
 * import (metadata-only). Mock: a plausible "found your default install"
 * result so the wizard is reviewable in a plain browser.
 */
export async function environmentScan(): Promise<DiscoveredInfo[]> {
  // The same two setups as the full scan's preview report.
  if (!hasBackend) return DEFAULT_SCAN_REPORT.installs;
  return invoke<DiscoveredInfo[]>("environment_scan");
}

/** Imports a discovered setup as a managed account (byte copy; source untouched). */
export async function accountImport(
  harness: string,
  sourcePath: string,
  label: string,
  plan: string | null,
  origin: string | null = null,
  degraded = false,
  inPlace = false,
): Promise<ImportResult> {
  if (!hasBackend) {
    // ADR-0018 test seam: a delay per import, so Stop and the progress
    // rows are reviewable in a plain browser.
    const delay = (window as unknown as { __AIJ_IMPORT_DELAY_MS__?: number }).__AIJ_IMPORT_DELAY_MS__;
    if (delay) await new Promise((resolve) => setTimeout(resolve, delay));
    // AJ-E304 test seam: substrings of source paths that reject with a
    // coded error, so the partial-failure accept flow (one item fails,
    // the rest complete) is reviewable in a plain browser.
    // AJ-E307 test seam: reject with the coded duplicate error.
    const duplicateLabel = (
      window as unknown as { __AIJ_IMPORT_DUPLICATE_LABEL__?: string }
    ).__AIJ_IMPORT_DUPLICATE_LABEL__;
    if (duplicateLabel !== undefined) {
      throw {
        code: "already_imported",
        retryable: false,
        user_message: `Already imported as «${duplicateLabel}» — open it from Accounts instead of importing again`,
      };
    }
    // AJ-E1022 test seam: reject with a given error (a moved folder…).
    const forced = (window as unknown as { __AIJ_IMPORT_ERROR__?: unknown }).__AIJ_IMPORT_ERROR__;
    if (forced !== undefined) throw forced;
    const failPaths =
      (window as unknown as { __AIJ_IMPORT_FAIL_PATHS__?: string[] })
        .__AIJ_IMPORT_FAIL_PATHS__ ?? [];
    // Copies can fail halfway; using a folder where it is copies nothing.
    for (const needle of inPlace ? [] : failPaths) {
      if (sourcePath.includes(needle)) {
        throw {
          code: "io",
          retryable: true,
          user_message: "Disk full — this one couldn't be copied.",
        };
      }
    }
    return {
      account: {
        id: `claude-${label.toLowerCase().replace(/\s+/g, "-")}`,
        provider: harness,
        label,
        plan,
        login_state: inPlace ? "logged_in" : "logged_out",
        cleanup_opt_in: false,
        renewal_date: null,
        preview_opt_in: false,
        in_place: inPlace,
      },
      files_copied: inPlace ? 0 : degraded ? 8 : 128,
      bytes_copied: inPlace ? 0 : degraded ? 6_000_000 : 46_000_000,
      folders: degraded
        ? [{ name: "settings.json", bytes: 60 }, { name: ".credentials.json", bytes: 40 }]
        : [
            { name: "projects/", bytes: 40_000_000 },
            { name: "settings.json", bytes: 60 },
            { name: ".credentials.json", bytes: 40 },
          ],
      degraded,
      skipped: degraded
        ? ["projects/ (37.3 GiB history — skipped at your choice)", "shell-snapshots/ (rebuildable cache)"]
        : ["shell-snapshots/ (rebuildable cache)", "statsig/ (rebuildable cache)"],
      login: inPlace ? "logged_in" : "logged_out",
      login_note: inPlace
        ? null
        : "Your Claude login lives in the macOS Keychain, locked to the original folder — it can't (and shouldn't) be moved. Sign in once inside the imported account; it takes a few seconds.",
    };
  }
  return invoke<ImportResult>("account_import", {
    harness,
    sourcePath,
    label,
    plan,
    origin,
    degraded,
    inPlace,
  });
}

/** AJ-E307 Req.3: inspect a candidate import source BEFORE any copy
 * (metadata-only): sizes, per-entry breakdown, evidence, login. */
export async function sourceInspect(
  harness: string,
  sourcePath: string,
): Promise<SourceInspectInfo> {
  if (!hasBackend) {
    return (
      (window as unknown as { __AIJ_SOURCE_INSPECT__?: SourceInspectInfo })
        .__AIJ_SOURCE_INSPECT__ ?? {
        total_bytes: 46_000_000,
        file_count: 128,
        folders: [
          { name: "projects/", bytes: 40_000_000 },
          { name: "settings.json", bytes: 60 },
          { name: ".credentials.json", bytes: 40 },
        ],
        truncated: false,
        ceiling_bytes: 2_147_483_648,
        is_config: true,
        already_managed: false,
        login: "logged_in",
        keychain_bound: true,
      }
    );
  }
  return invoke<SourceInspectInfo>("source_inspect", { harness, sourcePath });
}
