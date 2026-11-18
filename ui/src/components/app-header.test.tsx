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
  expect(line).toContain("@min-[960px]/header:flex");
  expect(line).not.toContain("xl:flex");
  cleanup();
  const paired = header(1);
  expect(paired.getByRole("button", { name: "Fleet" })).toBeTruthy();
  expect(paired.getByTestId("header-this-mac").className).toContain("@min-[1060px]/header:flex");
});

it("lights Fleet on a paired Mac's session, as its breadcrumb starts", () => {
  expect(navLit("fleet", null, "remote")).toBe(true);
  expect(navLit("home", null, "remote")).toBe(false);
});

it("is calm chrome (D-117 c2, D-118's mock): ‹ › only with somewhere to go, New session as accent words with no fill", async () => {
  const { fireEvent, render: draw, screen } = await import("@testing-library/react");
  const onBack = () => {};
  const view = draw(
    <AppHeader title="Home" global={null} page="home" pairedCount={0} cleanupCount={0} sessions={[]} onPick={() => {}} onOpenThisMac={() => {}} onNewSession={() => {}} back={{ canBack: false, canForward: false, onBack, onForward: onBack }} />,
  );
  // D-127: nothing to go back or forward to, so no greyed arrows.
  expect(view.queryByTestId("header-back-forward")).toBeNull();
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
