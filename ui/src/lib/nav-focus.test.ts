import { describe, expect, it } from "vitest";
import { focusPageSoon } from "./nav";

const frames = () => new Promise<void>((done) => requestAnimationFrame(() => requestAnimationFrame(() => requestAnimationFrame(() => done()))));

describe("a screen opened from a menu puts the keyboard on its page (§2.5)", () => {
  it("focuses the app's main once the screen has rendered", async () => {
    document.body.innerHTML = '<button id="menu">Your menu</button><main tabindex="-1"><h1>Processes</h1></main>';
    document.getElementById("menu")!.focus();
    focusPageSoon();
    await frames();
    expect(document.activeElement?.tagName).toBe("MAIN");
  });
});
