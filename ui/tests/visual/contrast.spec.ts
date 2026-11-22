/**
 * AJ-E804 Req.4 — text contrast, measured. Reads every text colour token
 * and every surface token from the running app in both themes (the
 * computed values, not the stylesheet's source) and computes the WCAG
 * 2.2 contrast ratio of each pair. Every text token must reach 4.5:1 on
 * every surface it can sit on; the table is written to the AJ-E804
 * evidence. Disabled controls are exempt (WCAG 1.4.3) and use opacity,
 * not a token.
 */
import { expect, test } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { allStatuses } from "./fixtures";

const TEXT = ["ink", "ink-2", "section", "muted", "faint", "accent", "accent-hi", "live", "success", "warning", "danger"] as const;
/** Floors above WCAG's 4.5:1: `muted` reaches 7:1 (D-142, COL-8). */
const MIN: Partial<Record<(typeof TEXT)[number], number>> = { muted: 7 };
/** D-117: each count badge's ink on the badge. */
const BADGES = [["count-ink", "count"], ["count-alarm-ink", "count-alarm"]] as const;
const SURFACES = ["canvas", "canvas-hi", "surface", "surface-2", "surface-3"] as const;
/** D-117 (selection): what a selected item's edge stands against — every
 * surface, the filled control its unselected segments show, and its own
 * raised fill — at 3:1 or more (WCAG 1.4.11), never colour alone (its words
 * are semibold too; the vitest checks that). */
const AROUND_SELECTED = [...SURFACES, "fill", "selected"] as const;
const OUT = resolve(process.cwd(), "../docs/qa/evidence/AJ-E804");

function channel(c: number): number {
  const s = c / 255;
  return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
}
function luminance([r, g, b]: number[]): number {
  return 0.2126 * channel(r!) + 0.7152 * channel(g!) + 0.0722 * channel(b!);
}
function ratio(a: number[], b: number[]): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi! + 0.05) / (lo! + 0.05);
}

/** APCA 0.0.98G |Lc| of text on a background (sRGB 0–255 channels). */
function apca(text: number[], background: number[]): number {
  const y = ([r, g, b]: number[]) => {
    const lum = 0.2126729 * (r! / 255) ** 2.4 + 0.7151522 * (g! / 255) ** 2.4 + 0.072175 * (b! / 255) ** 2.4;
    return lum > 0.022 ? lum : lum + (0.022 - lum) ** 1.414;
  };
  const [txt, bg] = [y(text), y(background)];
  const sapc = bg > txt ? (bg ** 0.56 - txt ** 0.57) * 1.14 : (bg ** 0.65 - txt ** 0.62) * 1.14;
  const lc = Math.abs(sapc) < 0.1 ? 0 : sapc > 0 ? sapc - 0.027 : sapc + 0.027;
  return Math.abs(lc * 100);
}

const rows: string[] = [];
const failures: string[] = [];

for (const theme of ["dark", "light"] as const) {
  test(`text tokens reach 4.5:1 on every surface — ${theme}`, async ({ page }) => {
    await page.addInitScript(
      ({ fixture, theme: t }) => {
        (window as unknown as Record<string, unknown>).__AIJ_FIXTURE__ = fixture;
        localStorage.setItem("aij.theme", t);
        localStorage.setItem("aij.onboarding.dismissed", "1");
        sessionStorage.setItem("aij.ready.skipped", "1");
      },
      { fixture: allStatuses, theme },
    );
    await page.goto("/");
    await expect(page.getByTestId("user-menu-button")).toBeVisible();
    // Resolve each token to sRGB through the browser itself.
    const colours = await page.evaluate(
      (names) => {
        const probe = document.createElement("span");
        document.body.appendChild(probe);
        const out: Record<string, number[]> = {};
        for (const name of names) {
          probe.style.color = `var(--color-${name})`;
          const m = getComputedStyle(probe).color.match(/[\d.]+/g) ?? [];
          out[name] = m.slice(0, 3).map(Number);
        }
        probe.remove();
        return out;
      },
      [...TEXT, ...SURFACES, ...BADGES.flat(), "fill", "selected", "selected-ink", "selected-ring"],
    );
    // D-117 (selection): the selected edge against everything unselected
    // around it, and the selected words on the selected fill.
    for (const around of AROUND_SELECTED) {
      const r = ratio(colours["selected-ring"]!, colours[around]!);
      if (r < 3) failures.push(`${theme} selected-ring vs ${around}: ${r.toFixed(2)}:1 (non-text, 3:1)`);
      rows.push(`| ${theme} | \`selected-ring\` vs \`${around}\` (non-text, 3:1) | ${r.toFixed(2)}${r < 3 ? " ✗" : ""} | | | | |`);
    }
    {
      const r = ratio(colours["selected-ink"]!, colours.selected!);
      if (r < 4.5) failures.push(`${theme} selected-ink on selected: ${r.toFixed(2)}:1`);
      rows.push(`| ${theme} | \`selected-ink\` on \`selected\` | ${r.toFixed(2)}${r < 4.5 ? " ✗" : ""} | | | | |`);
    }
    // And as drawn: every chosen segment on the page carries the selected
    // edge, 3:1 off its control's fill where its unselected siblings sit;
    // the unselected carry no edge.
    const drawn = await page.evaluate(() => {
      const rgb = (value: string) => (value.match(/rgba?\([^)]*\)/g) ?? []).map((c) => (c.match(/[\d.]+/g) ?? []).slice(0, 4).map(Number));
      const opaque = (element: Element | null): number[] => {
        for (let at = element; at; at = at.parentElement) {
          const [c] = rgb(getComputedStyle(at).backgroundColor);
          if (c && (c[3] === undefined || c[3] === 1)) return c.slice(0, 3);
        }
        return [0, 0, 0];
      };
      return [...document.querySelectorAll<HTMLElement>("[role='radiogroup']")]
        .filter((group) => group.getBoundingClientRect().width > 0)
        .map((group) => {
          const radios = [...group.querySelectorAll<HTMLElement>("[role='radio']")];
          const on = radios.find((radio) => radio.getAttribute("aria-checked") === "true");
          const off = radios.filter((radio) => radio !== on);
          const edge = on ? rgb(getComputedStyle(on).boxShadow).find((c) => c[3] === undefined || c[3] === 1) : undefined;
          return {
            name: group.getAttribute("aria-label") ?? "",
            edge: edge?.slice(0, 3) ?? null,
            weight: on ? Number(getComputedStyle(on).fontWeight) : 0,
            around: opaque(off[0] ?? group),
            offEdged: off.filter((radio) => rgb(getComputedStyle(radio).boxShadow).some((c) => c[3] === undefined || c[3] === 1)).length,
          };
        });
    });
    expect(drawn.length, "a segmented control on the page").toBeGreaterThan(0);
    for (const control of drawn) {
      expect(control.edge, `${theme} ${control.name}: the chosen segment's edge`).not.toBeNull();
      const r = ratio(control.edge!, control.around);
      rows.push(`| ${theme} | ${control.name}: chosen edge vs unselected (as drawn) | ${r.toFixed(2)}${r < 3 ? " ✗" : ""} | | | | |`);
      if (r < 3) failures.push(`${theme} ${control.name}: chosen edge vs unselected ${r.toFixed(2)}:1`);
      expect(control.weight, `${theme} ${control.name}: the chosen word semibold`).toBeGreaterThanOrEqual(600);
      expect(control.offEdged, `${theme} ${control.name}: unselected segments carry no edge`).toBe(0);
    }
    for (const [ink, badge] of BADGES) {
      const r = ratio(colours[ink]!, colours[badge]!);
      if (r < 4.5) failures.push(`${theme} ${ink} on ${badge}: ${r.toFixed(2)}:1`);
      rows.push(`| ${theme} | \`${ink}\` on \`${badge}\` | ${r.toFixed(2)}${r < 4.5 ? " ✗" : ""} | | | | |`);
    }
    // D-117: in dark, the alarm keeps its APCA floor on every surface
    // (warning Lc 60, danger Lc 52): never the quietest status colour.
    if (theme === "dark") {
      for (const bg of SURFACES) {
        const w = apca(colours.warning!, colours[bg]!);
        const d = apca(colours.danger!, colours[bg]!);
        if (w < 60) failures.push(`dark warning on ${bg}: Lc ${w.toFixed(1)}`);
        if (d < 52) failures.push(`dark danger on ${bg}: Lc ${d.toFixed(1)}`);
      }
    }
    for (const fg of TEXT) {
      // D-142 (GAPS B1, COL-8): labels and help in `muted` reach 7:1.
      const floor = MIN[fg] ?? 4.5;
      const cells = SURFACES.map((bg) => {
        const r = ratio(colours[fg]!, colours[bg]!);
        if (r < floor) failures.push(`${theme} ${fg} on ${bg}: ${r.toFixed(2)}:1 (${floor}:1)`);
        return `${r.toFixed(2)}${r < floor ? " ✗" : ""}`;
      });
      rows.push(`| ${theme} | \`${fg}\` | ${cells.join(" | ")} |`);
    }
    expect(failures.filter((f) => f.startsWith(theme)), "text below its floor (4.5:1; muted 7:1)").toEqual([]);
  });
}

test.afterAll(() => {
  // The table is written under `just a11y-gate` only.
  if (rows.length === 0 || process.env.AJ_A11Y_EVIDENCE !== "1") return;
  mkdirSync(OUT, { recursive: true });
  writeFileSync(
    resolve(OUT, "contrast.md"),
    [
      "# AJ-E804 — text contrast, measured",
      "",
      "Generated by `ui/tests/visual/contrast.spec.ts` from the computed",
      "token colours of the running app. WCAG 2.2 AA: 4.5:1 for text (all",
      "of the app's text is under the large-text size); `muted` 7:1 (D-142). ✗ = below its floor.",
      "",
      `| Theme | Text token | ${SURFACES.join(" | ")} |`,
      `|---|---|${SURFACES.map(() => "---").join("|")}|`,
      ...rows,
      "",
      failures.length === 0 ? "Verdict: **PASS** — every pair at or above its floor." : `Verdict: **FAIL** — ${failures.length} pairs below their floor.`,
      "",
    ].join("\n"),
  );
});
