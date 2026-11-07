/**
 * AJ-E3508: what a floor logo is drawn from, its print (the backend's
 * hash), and the fallbacks.
 */
import { describe, expect, it } from "vitest";

import { fnv1a, logoKeeper, logoParts, logoPrint, logosToDraw, ownInitials } from "./floor-logos";
import type { FloorApp } from "./ipc/floor";

function app(over: Partial<FloorApp> = {}): FloorApp {
  return {
    id: "cli:work", name: "Claude Code · Work Max", tool: "claude-code", host_app: null, logo: null, kind: "cli", family: "workshop", colour_slot: 2,
    account_id: "acct", initials: "WM", open: true, last_active_ms: 0, cpu_pct: 0, hot_since_ms: null, sessions: [], ...over,
  };
}

describe("floor logos (AJ-E3508)", () => {
  it("prints as the backend hashes (FNV-1a, 32-bit)", () => {
    expect(fnv1a("")).toBe("811c9dc5");
    expect(fnv1a("a")).toBe("e40c292c");
    expect(fnv1a("foobar")).toBe("bf9cf968");
  });

  it("a desktop App draws its own icon; an App with no Account its own initials", () => {
    expect(logoParts(app({ kind: "desktop", host_app: "claude-app" })).mark).toEqual({ kind: "icon", appId: "claude-app" });
    expect(logoParts(app({ initials: "" })).initials).toBe("CC");
    expect(ownInitials("Codex · Home")).toBe("CH");
  });

  it("a tool with no drawable mark writes its name in letters", () => {
    const parts = logoParts(app({ tool: "some-new-tool" }));
    expect(parts.mark.kind).toBe("letters");
  });

  it("a new colour, mark or Account name is a new print; a kept print isn't drawn again", () => {
    const base = logoPrint(logoParts(app()));
    expect(logoPrint(logoParts(app({ colour_slot: 3 })))).not.toBe(base);
    expect(logoPrint(logoParts(app({ initials: "PM" })))).not.toBe(base);
    expect(logosToDraw([app({ logo: base }), app({ id: "cli:other" })]).map((t) => t.app.id)).toEqual(["cli:other"]);
  });

  it("a logo the cache lost is drawn again; a failed one isn't retried", async () => {
    const put: string[] = [];
    const keep = logoKeeper(async (id) => {
      put.push(id);
      if (id === "cli:bad") throw new Error("refused");
    });
    const print = logoPrint(logoParts(app()));
    await keep([app()]);
    expect(put).toEqual(["cli:work"]);
    await keep([app({ logo: print })]);
    expect(put).toEqual(["cli:work"]);
    // the cache was cleared: the snapshot has no logo again
    await keep([app()]);
    expect(put).toEqual(["cli:work", "cli:work"]);
    await keep([app({ id: "cli:bad" })]);
    await keep([app({ id: "cli:bad" })]);
    expect(put.filter((id) => id === "cli:bad")).toHaveLength(1);
  });
});
