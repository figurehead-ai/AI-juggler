/**
 * The 0.3 re-render: This Mac's line gives way by the top bar's own width
 * (a container query), not the window's, and earlier when Fleet's tab is
 * in the bar too — at 1280 px with the list open the bar wrapped to two
 * rows once a Mac was paired.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { AppHeader } from "./AppHeader";
import { navLit } from "./MainNav";

afterEach(cleanup);

function header(pairedCount: number) {
  return render(
    <AppHeader title="Home" global={null} page="home" pairedCount={pairedCount} cleanupCount={0} sessions={[]} onPick={() => {}} onOpenThisMac={() => {}} onNewSession={() => {}} />,
  );
}

it("yields This Mac's line by the bar's width, sooner with Fleet's tab", () => {
  const alone = header(0);
  expect(alone.container.querySelector("header")!.className).toContain("@container/header");
  const line = alone.getByTestId("header-this-mac").className;
  // From 960 px of the bar's content: 1280 with the list open (976) keeps
  // it, as D-118's mock; with Bot's icon (D-136) the bar still fits one row
  // from 936 (measured in the 0.5 gate triage).
  expect(line).toContain("@min-[960px]/header:flex");
  expect(line).not.toContain("xl:flex");
  cleanup();
  const paired = header(1);
  expect(paired.getByRole("button", { name: "Fleet" })).toBeTruthy();
  expect(paired.getByTestId("header-this-mac").className).toContain("@min-[1160px]/header:flex");
});

it("lights Fleet on a paired Mac's session, as its breadcrumb starts", () => {
  expect(navLit("fleet", null, "remote")).toBe(true);
  expect(navLit("home", null, "remote")).toBe(false);
});

it("is calm chrome (D-117 c2, D-118's mock): ‹ › always there, greyed with nowhere to go (the tabs never move), New session as accent words with no fill", async () => {
  const { fireEvent, render: draw, screen } = await import("@testing-library/react");
  const onBack = () => {};
  const view = draw(
    <AppHeader title="Home" global={null} page="home" pairedCount={0} cleanupCount={0} sessions={[]} onPick={() => {}} onOpenThisMac={() => {}} onNewSession={() => {}} back={{ canBack: false, canForward: false, onBack, onForward: onBack }} />,
  );
  // Nothing to go back or forward to: both greyed, but there, so the tabs keep their place.
  expect(view.getByTestId("header-back").hasAttribute("disabled")).toBe(true);
  expect(view.getByTestId("header-forward").hasAttribute("disabled")).toBe(true);
  const start = screen.getByTestId("header-new-session");
  expect(start.className).toContain("text-accent-hi");
  expect(start.className).not.toMatch(/\bbg-accent/);
  fireEvent.click(start);
  cleanup();
  const again = draw(
    <AppHeader title="Home" global={null} page="home" pairedCount={0} cleanupCount={0} sessions={[]} onPick={() => {}} onOpenThisMac={() => {}} onNewSession={() => {}} back={{ canBack: true, canForward: false, onBack, onForward: onBack }} />,
  );
  expect(again.getByTestId("header-back")).toBeTruthy();
  expect(again.getByTestId("header-forward").hasAttribute("disabled")).toBe(true);
});

it("lights the current page as D-118's mock does: semibold words, no box, the others regular", () => {
  const view = render(
    <AppHeader title="Home" global={null} page="none" pairedCount={0} cleanupCount={0} sessions={[]} onPick={() => {}} onOpenThisMac={() => {}} onNewSession={() => {}} />,
  );
  const home = view.getByTestId("nav-home");
  expect(home.getAttribute("aria-current")).toBe("page");
  expect(home.className).toContain("font-semibold");
  expect(home.className).not.toMatch(/\bbg-selected\b|inset-ring/);
  const projects = view.getByTestId("nav-projects");
  expect(projects.className).toContain("font-normal");
  // Each tab keeps its semibold width, so lighting another moves none.
  expect(home.dataset.label).toBe("Home");
  expect(home.className).toContain("after:content-[attr(data-label)]");
});

it("keeps This Mac's room words in sight at 1280 with the list open (970 px of bar), whole or not at all", () => {
  const view = header(0);
  const words = view.getByTestId("machine-status-words");
  expect(words.className).toContain("@max-[969px]/header:hidden");
  // Never cut mid-word ("Plenty of r…"): they give way whole to the dot.
  expect(words.className).not.toContain("truncate");
  expect(words.className).toContain("whitespace-nowrap");
  expect(words.dataset.giveAt).toBe("1");
  expect(words.className).toContain("data-gone:hidden");
});
