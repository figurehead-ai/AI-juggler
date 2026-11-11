import { describe, expect, it } from "vitest";
import { HISTORY_LIMIT, emptyHistory, goBack, goForward, visit } from "./nav-history";

describe("Back and Forward (walk A3)", () => {
  it("goes back through the pages opened, and forward again", () => {
    // Home → a session → its App page.
    let history = visit(visit(emptyHistory<string>(), "home"), "session");
    const back = goBack(history, "app")!;
    expect(back.to).toBe("session");
    history = back.history;
    const again = goBack(history, "session")!;
    expect(again.to).toBe("home");
    expect(goBack(again.history, "home")).toBeNull();
    const ahead = goForward(again.history, "home")!;
    expect(ahead.to).toBe("session");
    expect(goForward(ahead.history, "session")!.to).toBe("app");
  });

  it("a new page after a Back ends Forward, as in a browser", () => {
    const history = visit(emptyHistory<string>(), "home");
    const back = goBack(history, "session")!;
    expect(back.history.forward).toEqual(["session"]);
    expect(visit(back.history, "home").forward).toEqual([]);
  });

  it("keeps at most its limit each way", () => {
    let history = emptyHistory<number>();
    for (let page = 0; page < HISTORY_LIMIT + 10; page += 1) history = visit(history, page);
    expect(history.back).toHaveLength(HISTORY_LIMIT);
    expect(history.back[0]).toBe(10);
  });
});
