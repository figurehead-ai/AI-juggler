// @vitest-environment node
/**
 * AJ-E1552 — no skeleton outside a read's loading state. A component that
 * draws a skeleton (`bg-skeleton`, a `…Skeleton` or `Placeholder`
 * component, `AppRowPending`) does it through `LoadingState`
 * (components/LoadingState.tsx), which draws it only while its read
 * loads, with "Still loading…" and a deadline (lib/loading). The
 * skeletons themselves, and the boundaries that draw one only while code
 * loads, are listed with why. Sites from before the primitive are a
 * ratchet: each may only lose skeletons, and leaves the list once it is
 * on the primitive; a new site can't be added to it.
 */
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const SRC = fileURLToPath(new URL("../../src", import.meta.url));
const MARKER = /bg-skeleton|<[A-Z][A-Za-z]*Skeleton\b|<AppRowPending|<Placeholder\b|<RowPlaceholders/g;

/** Skeletons themselves, and boundaries that draw one only while code loads. */
const BY_DESIGN: Record<string, string> = {
  "components/AppIcon.tsx": "an icon's pulse while its picture is read; at the deadline (useLoadDeadline) it shows its letters instead: an icon's slot has no room for words or Retry (AJ-E1552)",
  "components/ListSkeleton.tsx": "the list skeleton itself",
  "components/Placeholder.tsx": "the row placeholders themselves",
  "components/ScreenBoundary.tsx": "AreaSkeleton, drawn only as a lazy part's Suspense fallback (AJ-E1549)",
  "shell/screens.tsx": "the Privacy redirect's one frame (AJ-E1549)",
  "screens/projects/ProjectRow.tsx": "ProjectRowSkeleton, the row skeleton itself (drawn by ProjectsScreen through LoadingState)",
  "pages/pool/PoolPanel.tsx": "the Tasks and Messages tabs' list skeletons, drawn only as a lazy part's fallback while its code loads (AJ-E1552)",
};

/** Before the primitive (AJ-E1552): the skeletons each draws today. Only
 * falls; a file moved onto LoadingState leaves this list. */
const LEGACY: Record<string, number> = {
  "components/switch/SwitchHeader.tsx": 1,
};

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return sources(path);
    return path.endsWith(".tsx") && !/\.test\.tsx$/.test(path) ? [path] : [];
  });
}

const sites = sources(SRC)
  .map((path) => ({ file: relative(SRC, path).split("\\").join("/"), text: readFileSync(path, "utf8") }))
  .map(({ file, text }) => ({ file, count: text.match(MARKER)?.length ?? 0, onPrimitive: /from "[./]+(components\/)?LoadingState"/.test(text) }))
  .filter((site) => site.count > 0);

describe("skeletons are drawn only while a read loads (AJ-E1552)", () => {
  it("every skeleton site is on LoadingState, a skeleton by design, or a legacy site that only shrinks", () => {
    const problems: string[] = [];
    for (const site of sites) {
      if (site.onPrimitive || BY_DESIGN[site.file]) continue;
      const allowed = LEGACY[site.file];
      if (allowed === undefined) problems.push(`${site.file}: draws ${site.count} skeleton(s) outside LoadingState; draw it through LoadingState (components/LoadingState.tsx)`);
      else if (site.count > allowed) problems.push(`${site.file}: ${site.count} skeletons, was ${allowed}; a new one goes through LoadingState`);
    }
    expect(problems).toEqual([]);
  });

  it("the legacy list only shrinks: a site that left it, or lost skeletons, is taken off or lowered", () => {
    const now = new Map(sites.map((site) => [site.file, site]));
    const stale = Object.entries(LEGACY).flatMap(([file, allowed]) => {
      const site = now.get(file);
      if (!site || site.onPrimitive) return [`${file}: no longer a legacy site; take it off LEGACY`];
      return site.count < allowed ? [`${file}: ${site.count} skeletons now, lower LEGACY from ${allowed}`] : [];
    });
    expect(stale).toEqual([]);
  });

  it("the Account card, the 0.5.0 Nightly's stuck skeleton, is on the primitive", () => {
    expect(sites.find((site) => site.file === "screens/accounts/AccountCard.tsx")?.onPrimitive).toBe(true);
  });
});
