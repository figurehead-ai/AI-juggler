// @vitest-environment node
/**
 * AJ-E1546 — the clips' budget: every clip is there in both themes, with
 * its still poster, each clip with its poster at most 400 KB (they load
 * only when their card is on screen, but they ship in the app), and no
 * stray file beside them.
 */
import { readdirSync, statSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { CLIP_IDS } from "../../src/lib/clips";

const DIR = fileURLToPath(new URL("../../public/clips", import.meta.url));
const BUDGET = 400_000;

describe("the clips' size budget", () => {
  const files = readdirSync(DIR).sort();

  it("has each clip in both themes, with its poster, and nothing else", () => {
    const wanted = CLIP_IDS.flatMap((id) => ["dark", "light"].flatMap((theme) => [`${id}-${theme}.png`, `${id}-${theme}.webm`])).sort();
    expect(files).toEqual(wanted);
  });

  it.each(CLIP_IDS.flatMap((id) => [`${id}-dark`, `${id}-light`]))("%s is within the budget", (name) => {
    const bytes = statSync(resolve(DIR, `${name}.webm`)).size + statSync(resolve(DIR, `${name}.png`)).size;
    expect(bytes).toBeLessThanOrEqual(BUDGET);
  });
});
