/**
 * D-126 — the making screens' controls as a user meets them: a ToggleRow
 * is a switch and a CheckRow a checkbox, each named by its title and
 * described by its help, flipped from anywhere on the row and by Space
 * or Enter; RadioCards are a radio group of cards (one tab stop, the
 * arrows move and choose), the chosen one in the pool's chosen look;
 * a CheckCard is a checkbox card. A card that can't be chosen says why.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CheckCard, CheckRow, CHOSEN_CARD, RadioCard, RadioCards, RowList, ToggleRow } from ".";
import { SELECTED as POOL_SELECTED } from "../../pages/pool/selection";
import { IconBell } from "../icons-actions";

afterEach(cleanup);

function describedBy(element: Element): string {
  return (element.getAttribute("aria-describedby") ?? "")
    .split(" ")
    .filter(Boolean)
    .map((id) => document.getElementById(id)?.textContent ?? "")
    .join(" ");
}

describe("ToggleRow", () => {
  function Harness({ onChange }: { onChange?: (next: boolean) => void }) {
    const [on, setOn] = useState(false);
    return (
      <RowList label="Quiet hours">
        <ToggleRow
          icon={IconBell}
          title="Hold notifications at night"
          help="22:00 to 07:00 · nothing is dropped"
          checked={on}
          onChange={(next) => {
            setOn(next);
            onChange?.(next);
          }}
        />
      </RowList>
    );
  }

  it("is a switch named by its title and described by its help", () => {
    render(<Harness />);
    const toggle = screen.getByRole("switch", { name: "Hold notifications at night" });
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    expect(describedBy(toggle)).toBe("22:00 to 07:00 · nothing is dropped");
    expect(screen.getByRole("group", { name: "Quiet hours" })).toBeTruthy();
  });

  it("flips once from the switch and once from anywhere on its row", () => {
    const changed = vi.fn();
    render(<Harness onChange={changed} />);
    const toggle = screen.getByRole("switch", { name: "Hold notifications at night" });
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-checked")).toBe("true");
    fireEvent.click(screen.getByText("22:00 to 07:00 · nothing is dropped"));
    expect(toggle.getAttribute("aria-checked")).toBe("false");
    expect(changed.mock.calls).toEqual([[true], [false]]);
  });

  it("takes the accent on its chip while on, and says why it can't change", () => {
    const { rerender } = render(<ToggleRow icon={IconBell} title="Hold" checked onChange={() => {}} />);
    expect(screen.getByRole("switch", { name: "Hold" }).closest("label")!.querySelector("[aria-hidden='true']")!.className).toContain("bg-accent-soft");
    rerender(<ToggleRow icon={IconBell} title="Hold" checked={false} disabled reason="Notifications are off" onChange={() => {}} />);
    const toggle = screen.getByRole("switch", { name: "Hold" });
    expect(toggle.matches(":disabled")).toBe(true);
    expect(describedBy(toggle)).toBe("Notifications are off");
  });
});

describe("CheckRow", () => {
  function Harness() {
    const [on, setOn] = useState(true);
    return <CheckRow title="Copy settings from Work Max" help="MCP servers, permissions, status line" checked={on} onChange={setOn} />;
  }

  it("is a checkbox named by its title, ticked from its row, and by Enter", () => {
    render(<Harness />);
    const box = screen.getByRole("checkbox", { name: "Copy settings from Work Max" });
    expect(box.matches(":checked")).toBe(true);
    expect(describedBy(box)).toBe("MCP servers, permissions, status line");
    fireEvent.click(screen.getByText("MCP servers, permissions, status line"));
    expect(box.matches(":checked")).toBe(false);
    fireEvent.keyDown(box, { key: "Enter" });
    expect(box.matches(":checked")).toBe(true);
  });
});

describe("RadioCards", () => {
  function Harness({ disabled = false }: { disabled?: boolean }) {
    const [lead, setLead] = useState<string | null>("claude");
    return (
      <RadioCards label="Who leads" value={lead} onChange={setLead}>
        <RadioCard value="claude" title="Claude Code" sub="Installed · CLI" testId="card-claude" />
        <RadioCard value="codex" title="Codex" sub="Installed · desktop app and CLI" disabled={disabled} reason="Its app isn't installed" />
        <RadioCard value="none" title="No lead for now" sub="Pick one any time" />
      </RadioCards>
    );
  }
  const radio = (name: string) => screen.getByRole("radio", { name });
  const isChecked = (name: string) => radio(name).getAttribute("aria-checked") ?? String(radio(name).matches(":checked"));

  it("is a radio group of cards, the chosen one checked, in the pool's chosen look", () => {
    render(<Harness />);
    expect(screen.getByRole("radiogroup", { name: "Who leads" })).toBeTruthy();
    expect(isChecked("Claude Code")).toBe("true");
    expect(describedBy(radio("Claude Code"))).toBe("Installed · CLI");
    expect(CHOSEN_CARD).toBe(POOL_SELECTED);
    for (const part of CHOSEN_CARD.split(" ")) expect(screen.getByTestId("card-claude").className).toContain(part);
    expect(radio("Codex").closest("label")!.className).not.toContain("ring-accent");
  });

  it("is one native group (one tab stop); the arrows move and choose, Enter and a click choose", () => {
    render(<Harness />);
    const names = screen.getAllByRole("radio").map((one) => one.getAttribute("name"));
    expect(new Set(names).size).toBe(1);
    radio("Claude Code").focus();
    fireEvent.keyDown(radio("Claude Code"), { key: "ArrowDown" });
    expect(isChecked("Codex")).toBe("true");
    expect(document.activeElement).toBe(radio("Codex"));
    fireEvent.keyDown(radio("Codex"), { key: "End" });
    expect(isChecked("No lead for now")).toBe("true");
    fireEvent.click(screen.getByText("Installed · CLI"));
    expect(isChecked("Claude Code")).toBe("true");
    fireEvent.keyDown(radio("Codex"), { key: "Enter" });
    expect(isChecked("Codex")).toBe("true");
  });

  it("skips a card that can't be chosen, which says why", () => {
    render(<Harness disabled />);
    const codex = radio("Codex");
    expect(codex.matches(":disabled")).toBe(true);
    expect(describedBy(codex)).toBe("Installed · desktop app and CLI Its app isn't installed");
    fireEvent.click(screen.getByText("Installed · desktop app and CLI"));
    expect(isChecked("Codex")).toBe("false");
    radio("Claude Code").focus();
    fireEvent.keyDown(radio("Claude Code"), { key: "ArrowDown" });
    expect(isChecked("No lead for now")).toBe("true");
  });
});

describe("CheckCard", () => {
  it("is a checkbox card that a click flips", () => {
    function Harness() {
      const [on, setOn] = useState(false);
      return <CheckCard title="Claude" sub="Desktop app" layout="stack" checked={on} onChange={setOn} />;
    }
    render(<Harness />);
    const card = screen.getByRole("checkbox", { name: "Claude" });
    expect(card.matches(":checked")).toBe(false);
    fireEvent.click(screen.getByText("Desktop app"));
    expect(card.matches(":checked")).toBe(true);
    fireEvent.keyDown(card, { key: "Enter" });
    expect(card.matches(":checked")).toBe(false);
  });
});
