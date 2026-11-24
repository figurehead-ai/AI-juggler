/**
 * AJ-E1548: the 0.5.0 usability audit's word rules, one test each: one
 * name per thing, every mark said plainly, every limit shown only when
 * it matters (docs/qa/0.5.1-usability-audit.md, the numbers below).
 */
import { describe, expect, it } from "vitest";
import { render } from "@testing-library/react";
import { CONDITION_LINE, NOUN } from "../components/watches/watches-words";
import { levelWords } from "../components/MachineStatusLine";
import { LegendLevels } from "../components/LegendLevels";
import { WatchesHead } from "../components/watches/WatchesHead";
import { MARK_HINTS } from "./feed-words";
import { JOB_NAMES } from "../screens/settings/ai/models-words";
import { WORDS as BOT_WORDS } from "./bot-ui-words";

// The screens' own words, read as text (Vite's ?raw), for the word rules.
const SOURCES = import.meta.glob<string>("../{components,screens}/**/*.tsx", { query: "?raw", import: "default", eager: true });
const source = (path: string): string => {
  const text = SOURCES[path];
  if (text === undefined) throw new Error(`no source ${path}`);
  return text;
}

describe("AJ-E1548 #8 #10 #28: one meaning per word", () => {
  it("Finishes is a session ending on its own; a watch's turn end is said as a turn", () => {
    // Settings › Notifications and the feed: "A session finishes" is the session's end.
    expect(source("../components/NotificationSettings.tsx")).toContain('label: "A session finishes", hint: "It ended on its own, not stopped by you."');
    // Watches: the turn's end, never a bare "finish".
    expect(CONDITION_LINE.finishes).toBe("Its turn ends and it waits for you");
    expect(NOUN.finishes).toBe("A turn ending");
    // The backend's label and sentences: the shared echo fixture (watch-echo.json).
  });

  it("the bell and Processes say Needs you, as the rail and Home do", () => {
    expect(source("../components/NotificationCentre.tsx")).toContain("`Needs you · ${count}`");
    expect(source("../components/NotificationCentre.tsx")).not.toContain("`Needs action · ${count}`");
    expect(source("../screens/processes/Toolbar.tsx")).toContain('{ id: "attention", label: "Needs you" }');
  });
});

describe("AJ-E1548 #11 #12: the header names its subject; the marks read plainly", () => {
  it("the top bar says This Mac's state with its name, never a bare room", () => {
    expect(levelWords({ level: "plenty", words: "Plenty of room", cause: null, calm: true }).brief).toBe("This Mac: fine");
    expect(levelWords({ level: "tight", words: "Getting tight", cause: null, calm: false }).brief).toBe("This Mac: getting tight");
    expect(levelWords({ level: "tight", words: "Getting tight", cause: "swap almost full", calm: false }).brief).toBe("This Mac: tight, swap almost full");
    expect(levelWords({ level: "pressure", words: "Under memory pressure", cause: null, calm: false }).brief).toBe("This Mac: under pressure");
  });

  it("the legend draws the rail's own marks and says where they sit", () => {
    const view = render(<LegendLevels />);
    const marks = view.container.querySelectorAll(".level-mark");
    expect([...marks].map((m) => m.getAttribute("data-level"))).toEqual(["3", "2"]);
    expect(view.container.textContent).toContain("a red ✕ after the number");
    expect(view.container.textContent).toContain("an amber dot after the number");
    // The rail draws the count first, then its mark.
    const rail = source("../components/rail-groups.tsx");
    expect(rail.indexOf("{count}</span>\n      <span aria-hidden=\"true\" className=\"level-mark\"")).toBeGreaterThan(0);
  });
});

describe("AJ-E1548 #17: one machine word in the Unlink sheet", () => {
  it("the button, title, body and section all say it through machine-words", () => {
    const text = source("../components/FigureheadCloudParts.tsx");
    expect(text).not.toMatch(/this computer/i);
    expect(text).not.toMatch(/"[^"]*\bthis Mac\b[^"]*"/);
    expect(text).toContain("Unlink {THIS_MACHINE_IN_TEXT}…");
  });
});

describe("AJ-E1548 #19 #21: the feed's marks and scope say what they do", () => {
  it("each mark all says what it clears, and no two say the same", () => {
    expect(new Set(Object.values(MARK_HINTS)).size).toBe(3);
    expect(source("../components/HomeFeedFilters.tsx")).toContain("MARK_HINTS.read");
    expect(source("../components/HomeFeedFilters.tsx")).toContain("MARK_HINTS.reviewed");
    expect(source("../components/HomeFeedAway.tsx")).toContain("MARK_HINTS.seen");
  });

  it("the scope picker, closed, says it narrows the feed", () => {
    expect(source("../components/HomeFeedFilters.tsx")).toContain('prefix="Showing:"');
  });
});

describe("AJ-E1548 #23 #25: the Floor says what it is; the watch limit shows only near it", () => {
  it("the Floor's tooltip says what it is", () => {
    expect(source("../components/FloorButton.tsx")).toContain("title={`The Floor: your Apps as a live office (${keys.words})`}");
  });

  it("the count beside Watches shows from 40 of 50, said as watches", () => {
    const far = render(<WatchesHead count={39} limit={50} empty={false} onNew={() => {}} onAskBot={null} />);
    expect(far.queryByTestId("watches-count")).toBeNull();
    far.unmount();
    const near = render(<WatchesHead count={40} limit={50} empty={false} onNew={() => {}} onAskBot={null} />);
    expect(near.getByTestId("watches-count").textContent).toBe("40 of 50 watches");
  });
});

describe("AJ-E1548 #6: one name for Bot's written-answers switch", () => {
  it("Settings and Bot both call it Bot’s AI", () => {
    expect(JOB_NAMES.chat).toBe("Bot’s AI");
    expect(BOT_WORDS.betaSwitch).toBe(`${JOB_NAMES.chat} · Beta`);
    expect(Object.values(JOB_NAMES)).not.toContain("Bot answers");
  });
});
