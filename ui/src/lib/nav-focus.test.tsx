import { describe, expect, it } from "vitest";
import { fireEvent, render } from "@testing-library/react";
import { focusPageIfLost, focusPageSoon } from "./nav";
import { SkipLink } from "../components/SkipLink";

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

describe("a page opened from something that went with the old page (the 0.2 walk, F11)", () => {
  it("puts the keyboard on the new page, never the window's body", async () => {
    document.body.innerHTML = '<div id="home"><button id="row">A row on Home</button></div><main tabindex="-1"><h1>Session</h1></main>';
    document.getElementById("row")!.focus();
    document.getElementById("home")!.remove(); // Home made way for the page
    focusPageIfLost();
    await frames();
    expect(document.activeElement?.tagName).toBe("MAIN");
  });

  it("leaves focus that is still somewhere (a rail row, the terminal)", async () => {
    document.body.innerHTML = '<button id="rail-row">A rail row</button><main tabindex="-1"><h1>Session</h1></main>';
    document.getElementById("rail-row")!.focus();
    focusPageIfLost();
    await frames();
    expect(document.activeElement?.id).toBe("rail-row");
  });

  it("the skip link is the way past the rail: it puts the keyboard on the page", () => {
    document.body.innerHTML = '<main tabindex="-1"><h1>Home</h1></main>';
    const view = render(<SkipLink />);
    const link = view.getByRole("button", { name: "Skip to page" });
    fireEvent.click(link);
    expect(document.activeElement?.tagName).toBe("MAIN");
    view.unmount();
  });
});
