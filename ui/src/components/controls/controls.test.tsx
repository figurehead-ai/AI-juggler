/**
 * 0.3 — the shared controls as a user meets them: a Checkbox and Radio
 * that stay native to keyboards, labels and screen readers, say why
 * they're greyed, and draw in the accent; a Picker that opens a menu on
 * the chosen line, picks with a click or the keys, and closes on Escape
 * without closing the sheet it sits in; a choice line on one grid; a
 * segmented control moved by the arrow keys; the app's disclosure.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useEffect, useRef, useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Checkbox, ChoiceOption, Disclosure, Picker, Radio, Segmented } from ".";
import { Modal } from "../form";
import { anchoredEdge, menuEdge, menuPoint, type MenuAt } from "../RowMenu";

afterEach(cleanup);

/** The words an element's aria-describedby points at. */
function describedBy(element: Element): string {
  return (element.getAttribute("aria-describedby") ?? "")
    .split(" ")
    .filter(Boolean)
    .map((id) => document.getElementById(id)?.textContent ?? "")
    .join(" ");
}

describe("Checkbox and Radio", () => {
  it("are the native control, drawn by the shared style", () => {
    const onChange = vi.fn();
    render(
      <label>
        <Checkbox checked={false} onChange={onChange} data-testid="box" /> Keep it
      </label>,
    );
    const box = screen.getByLabelText("Keep it");
    expect(box.getAttribute("type")).toBe("checkbox");
    expect(box.className).toContain("aj-check");
    fireEvent.click(box);
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("says why it is greyed", () => {
    render(<Radio name="r" disabled reason="Make a pool first" aria-label="Pool" />);
    const radio = screen.getByRole("radio", { name: "Pool" });
    expect((radio as HTMLInputElement).disabled).toBe(true);
    expect(describedBy(radio)).toBe("Make a pool first");
    expect(radio.getAttribute("title")).toBe("Make a pool first");
  });

  it("shows some of a group as indeterminate", () => {
    render(<Checkbox aria-label="All" indeterminate checked={false} onChange={() => {}} />);
    expect((screen.getByRole("checkbox", { name: "All" }) as HTMLInputElement).indeterminate).toBe(true);
  });
});

describe("ChoiceOption", () => {
  it("is named by its label and described by its hint, with its reason when greyed", () => {
    render(
      <>
        <ChoiceOption type="radio" name="what" label="Just the session" hint="Its own processes" checked onChange={() => {}} />
        <ChoiceOption type="radio" name="what" label="The whole app" hint="Every session in it" disabled reason="It runs work you didn't start" />
      </>,
    );
    const first = screen.getByRole("radio", { name: "Just the session" });
    expect((first as HTMLInputElement).checked).toBe(true);
    expect(describedBy(first)).toBe("Its own processes");
    const second = screen.getByRole("radio", { name: "The whole app" });
    expect((second as HTMLInputElement).disabled).toBe(true);
    expect(describedBy(second)).toBe("Every session in it It runs work you didn't start");
    expect(screen.getByText("It runs work you didn't start")).toBeTruthy();
  });
});

function PickerHarness({ onPick }: { onPick?: (value: string) => void }) {
  const [value, setValue] = useState("latest");
  return (
    <Picker
      ariaLabel="Sort by"
      testId="sort"
      value={value}
      onChange={(next) => {
        setValue(next);
        onPick?.(next);
      }}
      options={[
        { value: "latest", label: "Latest activity", hint: "Newest first" },
        { value: "name", label: "Name" },
        { value: "cost", label: "Cost", disabled: true, reason: "No prices for this tool" },
      ]}
    />
  );
}

describe("Picker", () => {
  it("shows the value, opens on the chosen line, and picks", () => {
    const onPick = vi.fn();
    render(<PickerHarness onPick={onPick} />);
    const button = screen.getByRole("button", { name: "Sort by: Latest activity" });
    expect(button.getAttribute("aria-haspopup")).toBe("menu");
    expect(button.getAttribute("data-value")).toBe("latest");
    fireEvent.click(button);
    expect(button.getAttribute("aria-expanded")).toBe("true");
    const chosen = screen.getByRole("menuitemradio", { name: /Latest activity/ });
    expect(chosen.getAttribute("aria-checked")).toBe("true");
    expect(document.activeElement).toBe(chosen);
    const greyed = screen.getByRole("menuitemradio", { name: /Cost/ });
    expect((greyed as HTMLInputElement).disabled).toBe(true);
    expect(describedBy(greyed)).toBe("No prices for this tool");
    fireEvent.click(screen.getByTestId("sort-name"));
    expect(onPick).toHaveBeenCalledWith("name");
    expect(screen.queryByRole("menu")).toBeNull();
    expect(screen.getByRole("button", { name: "Sort by: Name" })).toBeTruthy();
  });

  it("opens from the keyboard, moves with the arrows, and Escape closes only the menu", () => {
    const onClose = vi.fn();
    render(
      <Modal title="A sheet" onClose={onClose}>
        <PickerHarness />
      </Modal>,
    );
    const button = screen.getByRole("button", { name: /Sort by/ });
    fireEvent.keyDown(button, { key: "ArrowDown" });
    expect(screen.getByRole("menu")).toBeTruthy();
    fireEvent.keyDown(document.activeElement!, { key: "ArrowDown" });
    expect(document.activeElement?.textContent).toBe("Name");
    fireEvent.keyDown(document.activeElement!, { key: "Escape" });
    expect(screen.queryByRole("menu")).toBeNull();
    expect(onClose).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(button);
  });

  it("picks inside a popover that closes on a press outside it", () => {
    const closed = vi.fn();
    function Popover() {
      const box = useRef<HTMLDivElement | null>(null);
      useEffect(() => {
        const down = (event: MouseEvent) => {
          if (!box.current?.contains(event.target as Node)) closed();
        };
        document.addEventListener("mousedown", down);
        return () => document.removeEventListener("mousedown", down);
      }, []);
      return (
        <div ref={box}>
          <PickerHarness />
        </div>
      );
    }
    render(<Popover />);
    fireEvent.click(screen.getByRole("button", { name: /Sort by/ }));
    const line = screen.getByTestId("sort-name");
    fireEvent.mouseDown(line);
    fireEvent.click(line);
    expect(closed).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Sort by: Name" })).toBeTruthy();
  });

  it("is greyed with a reason", () => {
    render(<Picker ariaLabel="Account" value="a" options={[{ value: "a", label: "Work" }]} onChange={() => {}} disabled reason="Only one account" />);
    const button = screen.getByRole("button", { name: "Account: Work" });
    expect((button as HTMLInputElement).disabled).toBe(true);
    expect(describedBy(button)).toBe("Only one account");
  });
});

describe("a menu from a button", () => {
  it("hangs below it, flips to its other side when there's no room, and keeps 8 px from the window's edge", () => {
    // Room below: at the point.
    expect(anchoredEdge(100, 60, 200, 800)).toBe(100);
    // No room below: it ends at the button's top (never over the button).
    expect(anchoredEdge(700, 660, 200, 800)).toBe(460);
    // No room either side: as far in as the window allows, 8 px in.
    expect(anchoredEdge(700, 150, 200, 800)).toBe(592);
    // A pointer's menu keeps the Mac's 4 px.
    expect(menuEdge(700, 200, 800)).toBe(500);
  });

  it("a click on a button gives the button's edges, a right-click the pointer", () => {
    let at: MenuAt | null = null;
    render(
      <button type="button" onClick={(event) => (at = menuPoint(event))} onContextMenu={(event) => (at = menuPoint(event))}>
        ⋯
      </button>,
    );
    const button = screen.getByRole("button");
    button.getBoundingClientRect = () => ({ left: 900, right: 930, top: 40, bottom: 70, width: 30, height: 30, x: 900, y: 40, toJSON: () => ({}) });
    fireEvent.click(button, { clientX: 915, clientY: 55 });
    expect(at).toEqual({ x: 900, y: 74, above: 36, right: 930 });
    fireEvent.contextMenu(button, { clientX: 915, clientY: 55, detail: 1 });
    expect(at).toEqual({ x: 915, y: 55 });
  });
});

describe("Segmented", () => {
  function Harness() {
    const [value, setValue] = useState<"place" | "project" | "account">("place");
    return (
      <Segmented
        label="Group sessions by"
        testId="seg"
        value={value}
        onChange={setValue}
        segments={[
          { value: "place", label: "App" },
          { value: "project", label: "Project" },
          { value: "account", label: "Account" },
        ]}
      />
    );
  }

  it("is a radio group with one tab stop that the arrows move", () => {
    render(<Harness />);
    const group = screen.getByRole("radiogroup", { name: "Group sessions by" });
    const app = screen.getByRole("radio", { name: "App" });
    expect(app.getAttribute("aria-checked")).toBe("true");
    expect(screen.getByRole("radio", { name: "Project" }).getAttribute("tabindex")).toBe("-1");
    fireEvent.keyDown(group, { key: "ArrowRight" });
    expect(screen.getByRole("radio", { name: "Project" }).getAttribute("aria-checked")).toBe("true");
    fireEvent.keyDown(group, { key: "ArrowLeft" });
    fireEvent.keyDown(group, { key: "ArrowLeft" });
    expect(screen.getByRole("radio", { name: "Account" }).getAttribute("aria-checked")).toBe("true");
    fireEvent.click(screen.getByTestId("seg-place"));
    expect(app.getAttribute("aria-checked")).toBe("true");
  });
});

describe("Disclosure", () => {
  it("is a details with the app's chevron, not the browser's triangle", () => {
    render(
      <Disclosure summary="Terminal" testId="d">
        <p>Inside</p>
      </Disclosure>,
    );
    const summary = screen.getByText("Terminal").closest("summary")!;
    expect(summary.className).toContain("list-none");
    expect(summary.querySelector("svg")).not.toBeNull();
  });
});

describe("Segmented, sized by its words (the 0.3 re-render)", () => {
  it("lets each segment start at its word's width, so a long one isn't cut beside short ones", () => {
    const view = render(
      <Segmented label="Group sessions by" value="place" onChange={() => {}} segments={[{ value: "place", label: "App" }, { value: "project", label: "Project" }, { value: "account", label: "Account" }, { value: "pool", label: "Pool" }]} />,
    );
    for (const radio of view.getAllByRole("radio")) {
      expect(radio.className).toContain("flex-auto");
      expect(radio.className).not.toContain("flex-1");
    }
  });
});
