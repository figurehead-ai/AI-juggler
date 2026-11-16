/**
 * D-126 — the one time field, as DateField: typed as 24-hour text (the
 * colon put in), stepped with the arrows, handed over only whole and on
 * a commit; a partial or impossible time saves nothing and says why;
 * Escape puts the saved value back. DateField takes the same field look.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DateField, TimeField } from ".";
import { readTime, shapeTime, stepTime } from "./TimeField";
import { asInput } from "../../test/fixtures";

afterEach(cleanup);

describe("reading a time", () => {
  it("takes whole times in their usual shapes and nothing else", () => {
    expect(readTime("7")).toEqual({ kind: "time", hhmm: "07:00" });
    expect(readTime("22:00")).toEqual({ kind: "time", hhmm: "22:00" });
    expect(readTime("0730")).toEqual({ kind: "time", hhmm: "07:30" });
    expect(readTime("730")).toEqual({ kind: "time", hhmm: "07:30" });
    expect(readTime("7:3").kind).toBe("partial");
    expect(readTime("25:00").kind).toBe("invalid");
    expect(readTime("7:61").kind).toBe("invalid");
    expect(readTime("soon").kind).toBe("invalid");
    expect(readTime(" ").kind).toBe("empty");
  });

  it("puts the colon in as the hour is whole, and steps round the clock", () => {
    expect(shapeTime("22")).toBe("22:");
    expect(shapeTime("223")).toBe("22:3");
    expect(stepTime("23:50", 15)).toBe("00:05");
    expect(stepTime("00:00", -60)).toBe("23:00");
  });
});

describe("TimeField", () => {
  function Harness({ onCommit }: { onCommit: (next: string | null) => void }) {
    const [value, setValue] = useState<string | null>("22:00");
    return (
      <TimeField
        ariaLabel="Quiet from"
        value={value}
        testId="from"
        onCommit={(next) => {
          setValue(next);
          onCommit(next);
        }}
      />
    );
  }

  it("is a text box named by its label; a typed time saves on Enter, once", async () => {
    const commit = vi.fn();
    render(<Harness onCommit={commit} />);
    const box = screen.getByRole("textbox", { name: "Quiet from" });
    await userEvent.clear(box);
    await userEvent.type(box, "0730");
    expect(asInput(box).value).toBe("07:30");
    fireEvent.keyDown(box, { key: "Enter" });
    fireEvent.blur(box);
    expect(commit.mock.calls).toEqual([["07:30"]]);
  });

  it("saves nothing for a partial time and says why; Escape brings the saved one back", async () => {
    const commit = vi.fn();
    render(<Harness onCommit={commit} />);
    const box = screen.getByRole("textbox", { name: "Quiet from" });
    await userEvent.clear(box);
    await userEvent.type(box, "7:3");
    fireEvent.keyDown(box, { key: "Enter" });
    expect(commit).not.toHaveBeenCalled();
    expect(screen.getByRole("alert").textContent).toContain("Not a whole time yet");
    expect(box.getAttribute("aria-invalid")).toBe("true");
    fireEvent.keyDown(box, { key: "Escape" });
    expect(asInput(box).value).toBe("22:00");
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("steps with the arrows (Shift for an hour), saving each step", () => {
    const commit = vi.fn();
    render(<Harness onCommit={commit} />);
    const box = screen.getByRole("textbox", { name: "Quiet from" });
    fireEvent.keyDown(box, { key: "ArrowUp" });
    fireEvent.keyDown(box, { key: "ArrowDown", shiftKey: true });
    expect(commit.mock.calls).toEqual([["22:15"], ["21:15"]]);
  });
});

describe("one field look", () => {
  it("draws DateField and TimeField alike: filled, a hairline edge, the kit's 36 px at lg", () => {
    render(
      <>
        <DateField ariaLabel="Renews on" value="2026-12-12" onCommit={() => {}} size="lg" />
        <TimeField ariaLabel="Quiet to" value="07:00" onCommit={() => {}} size="lg" />
      </>,
    );
    for (const name of ["Renews on", "Quiet to"]) {
      const shell = screen.getByRole("textbox", { name }).parentElement!;
      expect(shell.className).toContain("bg-fill");
      expect(shell.className).toContain("inset-ring-hairline/75");
      expect(shell.className).toContain("h-9");
    }
    expect(screen.getByRole("button", { name: "Choose from a calendar" })).toBeTruthy();
  });
});
