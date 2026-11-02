import { afterEach, describe, expect, it, vi } from "vitest";
import { openWhenListed } from "./dev-open";

afterEach(() => {
  document.body.innerHTML = "";
  vi.useRealTimers();
});

describe("QA: a Dev window opened at ?open=<id>", () => {
  it("clicks the session's row once it's listed", () => {
    vi.useFakeTimers();
    const clicked = vi.fn();
    openWhenListed("abc-123", 10);
    const row = document.createElement("button");
    row.dataset.conversationId = "abc-123";
    row.addEventListener("click", clicked);
    document.body.append(row);
    vi.advanceTimersByTime(20);
    expect(clicked).toHaveBeenCalledTimes(1);
  });

  it("never looks for an id that isn't plain", () => {
    const spy = vi.spyOn(document, "querySelector");
    openWhenListed('x"],[onclick');
    expect(spy).not.toHaveBeenCalled();
  });
});
