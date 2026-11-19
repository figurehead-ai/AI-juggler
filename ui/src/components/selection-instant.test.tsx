/**
 * The 0.4.6 RC walks A8 and K2: a page's tabs and the top nav lit their
 * chosen one by fading its colour in; WebKit pauses a fade in a window
 * that isn't shown, so the old tab stayed lit over the new panel. A
 * control that says what's chosen lights it at once, never faded.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { PageShell } from "../pages/PageShell";
import { MainNav } from "./MainNav";
import { Segmented } from "./controls/Segmented";

afterEach(cleanup);

const faded = (elements: Iterable<Element>) => [...elements].filter((element) => /\btransition(-colors|-all)?\b/.test(element.className)).map((element) => element.textContent);

describe("a chosen tab, nav item or segment is never faded in", () => {
  it("a page's tabs", () => {
    const view = render(
      <PageShell
        kind="app"
        tabs={[
          { id: "overview", label: "Overview", source: "" },
          { id: "settings", label: "Settings", source: "" },
        ]}
        header={<h1>App</h1>}
        render={(tab) => <p>{tab}</p>}
      />,
    );
    expect(faded(view.getAllByRole("tab"))).toEqual([]);
  });

  it("the top nav", () => {
    const view = render(
      <MainNav
        primary={[
          { id: "home", label: "Home" },
          { id: "projects", label: "Projects" },
        ]}
        global="projects"
        page="none"
        pairedCount={0}
        onPick={() => {}}
      />,
    );
    expect(faded(view.getByTestId("main-nav").querySelectorAll("button"))).toEqual([]);
  });

  it("a segmented choice", () => {
    const view = render(
      <Segmented
        label="Group by"
        value="app"
        segments={[
          { value: "app", label: "App" },
          { value: "project", label: "Project" },
        ]}
        onChange={() => {}}
      />,
    );
    expect(faded(view.getAllByRole("radio"))).toEqual([]);
  });
});
