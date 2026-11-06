/**
 * AJ-E3106 Req.3–4: each tool's own marks. Codex's final_answer is a
 * turn's reply, its commentary folded with the work even after it; a
 * step's own description shows, its command beside it.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { Steps, foldOf } from "./TurnWork";
import { blockOf } from "../../preview/conversation-feed";
import type { TurnInfo } from "../../types/generated/TurnInfo";

afterEach(() => cleanup());

const turn = (blocks: ReturnType<typeof blockOf>[]): TurnInfo => ({ id: "0:1", revision: 0, offset: 1, open: false, speaker: "assistant", blocks, blocks_hidden: 0, at_ms: null, model: null }) as unknown as TurnInfo;

describe("AJ-E3106 the tools' own marks", () => {
  it("Codex's final answer is the reply, its commentary work, even after it", () => {
    const blocks = [blockOf({ kind: "text", text: "Looking first." }), blockOf({ kind: "text", text: "Refunds are fixed.", final_answer: true }), blockOf({ kind: "text", text: "A note on the logs." })];
    const { reply, work } = foldOf(turn(blocks));
    expect(reply?.text).toBe("Refunds are fixed.");
    expect(work.map((block) => block.text)).toEqual(["Looking first.", "A note on the logs."]);
  });

  it("with no mark, the last words are the reply, as before", () => {
    const { reply, work } = foldOf(turn([blockOf({ kind: "steps", steps: ["ran \"ls\""] }), blockOf({ kind: "text", text: "Done." })]));
    expect(reply?.text).toBe("Done.");
    expect(work).toHaveLength(1);
  });

  it("a step shows its own description with its command beside it", () => {
    const call = { tool: "Bash", line: 'ran "just preflight"', description: "Run the full preflight", command: "just preflight", file_path: null, path: null, pattern: null, query: null, subagent_type: null, done: true, failed: false, result_kind: null, at_ms: null };
    render(<Steps block={blockOf({ kind: "steps", steps: ['ran "just preflight"'], calls: [call] })} />);
    fireEvent.click(screen.getByRole("button"));
    const line = screen.getByTestId("last-turn-step-described");
    expect(line.textContent).toContain("Run the full preflight");
    expect(line.textContent).toContain('ran "just preflight"');
  });
});
