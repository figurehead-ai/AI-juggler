/**
 * 0.5 betas only (D-117, AJ-E3601 R5) — a hidden comparison look for
 * side-by-side walks: `localStorage["aij.attention"] = "classic"` loads
 * attention-classic.css (the 0.4 colours over the calm grammar). Nothing
 * else reads it: no Settings, no baselines, one calm default otherwise.
 * AJ-E3617 deletes this file, the stylesheet and its call in main.tsx
 * before 0.5 goes Stable; tests/bundle/attention-classic-beta-only.test.ts
 * fails once a 0.5 Stable sign-off is recorded while they're still here.
 */
export const CLASSIC_KEY = "aij.attention";

/** A 0.5 build (beta or nightly; the version moves to Stable unchanged,
 * which the sign-off test catches), a Dev build, or the design preview. */
export function classicAllowed(version: string | null | undefined, edition?: string | null): boolean {
  return /^0\.5\.\d+/.test(version ?? "") || edition === "dev" || version === "0.0.0-preview";
}

/** The flag, read guarded (a private window, blocked storage: off). */
export function wantsClassic(): boolean {
  try {
    return localStorage.getItem(CLASSIC_KEY) === "classic";
  } catch {
    return false;
  }
}

/** Puts the classic look on when it's both allowed and asked for: the
 * stylesheet loads then, never otherwise. */
export async function applyAttentionMode(info: { version: string; edition: string } | null): Promise<boolean> {
  if (!info || !classicAllowed(info.version, info.edition) || !wantsClassic()) return false;
  await import("../attention-classic.css");
  document.documentElement.dataset.attention = "classic";
  return true;
}
