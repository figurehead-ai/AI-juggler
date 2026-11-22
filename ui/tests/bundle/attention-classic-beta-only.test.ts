// @vitest-environment node
/**
 * D-117 (AJ-E3601 R5, AJ-E3617): the hidden classic look lives through the
 * 0.5 beta only. Once a 0.5 Stable is signed off (its record in
 * docs/qa/signoffs/0.5.*.md, scripts/signoff_record.py), its code must be
 * gone: this fails while the flag's files are still in the tree.
 */
import { existsSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";

const root = (path: string) => fileURLToPath(new URL(`../../../${path}`, import.meta.url));
const FLAG_FILES = ["ui/src/lib/attention-classic.ts", "ui/src/attention-classic.css"];

it("the classic look is gone once 0.5 is Stable", () => {
  const signoffs = existsSync(root("docs/qa/signoffs")) ? readdirSync(root("docs/qa/signoffs")) : [];
  const stable05 = signoffs.filter((name) => /^0\.5\.\d+\.md$/.test(name));
  const present = FLAG_FILES.filter((path) => existsSync(root(path)));
  if (stable05.length > 0) expect(present, `0.5 is Stable (${stable05.join(", ")}): remove the classic flag (AJ-E3617)`).toEqual([]);
  else expect(present.length).toBeGreaterThanOrEqual(0);
});
