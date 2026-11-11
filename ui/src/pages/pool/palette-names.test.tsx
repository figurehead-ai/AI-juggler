/**
 * D-96 D on the canvas (the rc27 pool pass): four untitled Lab sessions in
 * one folder, started in the same minutes, all read "team · Lab · started
 * 9:2x AM" in Add a session. Alike names are told apart, as the nodes are.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { Palette } from "./Palette";
import { instance } from "../../test/fixtures";

afterEach(cleanup);

it("Add a session tells alike untitled sessions apart", () => {
  const at = new Date(2026, 10, 10, 9, 25).getTime();
  const lab = (id: string, started: number) => instance({ id, cwd: "/Users/dev/team", account_label: "Lab", started_at_ms: started });
  const sessions = [lab("a", at), lab("b", at + 20_000), lab("c", at - 60_000), lab("d", at - 50_000)];
  const view = render(<Palette sessions={sessions} projects={[]} removing={null} open onOpen={vi.fn()} onStartDrag={vi.fn()} onAdd={vi.fn()} />);
  const names = view.getAllByTestId("canvas-palette-item").map((item) => item.querySelector("span.text-ink")?.textContent ?? "");
  expect(names).toHaveLength(4);
  expect(new Set(names).size).toBe(4);
  expect(names.every((name) => /\(\d\)$/.test(name))).toBe(true);
});
