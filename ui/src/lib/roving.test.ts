import { describe, expect, it } from "vitest";
import { rove } from "./roving";

describe("the rail's list on the Tab path, kept after each render", () => {
  // AJ-E805: it runs after every render of the rail, and wrote every
  // control's mark again each time (about 2,000 writes a minute with 10
  // sessions at work). A render that changes nothing writes nothing.
  it("writes only what changed", async () => {
    const root = document.createElement("div");
    root.innerHTML = [1, 2, 3].map((n) => `<div data-rail-row><button data-rail-item="row-${n}">Row ${n}</button><button>More</button></div>`).join("");
    document.body.append(root);
    const remembered = rove(root, null);
    const stops = [...root.querySelectorAll<HTMLElement>("button")].map((button) => button.tabIndex);
    expect(stops).toEqual([0, 0, -1, -1, -1, -1]);
    const writes: MutationRecord[] = [];
    const watch = new MutationObserver((records) => writes.push(...records));
    watch.observe(root, { subtree: true, attributes: true });
    rove(root, remembered);
    rove(root, remembered);
    await Promise.resolve();
    watch.disconnect();
    expect(writes).toEqual([]);
    root.remove();
  });
});
