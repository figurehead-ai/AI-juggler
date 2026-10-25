/**
 * Lab 2 vitest: a tool's folder-trust question is asked in AI Juggler, in
 * the tool's own words, once per folder. The click is carried, and nothing
 * is answered by default: Cancel stops the launch.
 */
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { useState, type JSX } from "react";
import { useFolderTrust } from "./FolderTrustAsk";
import type { FolderTrustInfo } from "../lib/ipc/runtime";

type Seams = { __AIJ_TRUST__?: FolderTrustInfo; __AIJ_RUNTIME_CALLS__?: string[] };
const seams = () => window as unknown as Seams;
const calls = () => seams().__AIJ_RUNTIME_CALLS__ ?? [];

const codex: FolderTrustInfo = {
  folder: "/Users/me/lab",
  question: "Trust this folder?",
  trust: "Trust and continue",
  restrict: "Open restricted",
  note: "Codex asks this once per folder. AI Juggler keeps your answer and passes it to each Codex session here, so Codex's config.toml is never changed.",
  choice: null,
};

function Launcher(): JSX.Element {
  const trust = useFolderTrust();
  const [went, setWent] = useState<string>("");
  return (
    <div>
      <button onClick={() => void trust.confirm("p1", "/Users/me/lab").then((go) => setWent(go ? "went" : "stopped"))}>Start</button>
      <span data-testid="went">{went}</span>
      {trust.element}
    </div>
  );
}

beforeEach(() => {
  delete seams().__AIJ_RUNTIME_CALLS__;
});
afterEach(() => {
  cleanup();
  delete seams().__AIJ_TRUST__;
});

describe("a tool's folder-trust question", () => {
  it("is asked in the tool's words, and the click is kept", async () => {
    seams().__AIJ_TRUST__ = codex;
    const view = render(<Launcher />);
    await act(async () => fireEvent.click(view.getByText("Start")));
    const sheet = view.getByTestId("folder-trust");
    expect(sheet.textContent).toContain("Trust this folder?");
    expect(sheet.textContent).toContain("/Users/me/lab");
    expect(sheet.textContent).toContain("config.toml is never changed");
    expect(calls()).toEqual([]);
    await act(async () => fireEvent.click(view.getByTestId("folder-trust-trust")));
    expect(calls()).toEqual(["trust:p1:trusted"]);
    expect(view.getByTestId("went").textContent).toBe("went");
    expect(view.queryByTestId("folder-trust")).toBeNull();
  });

  it("Open restricted keeps that; Cancel stops the launch and keeps nothing", async () => {
    seams().__AIJ_TRUST__ = codex;
    const view = render(<Launcher />);
    await act(async () => fireEvent.click(view.getByText("Start")));
    await act(async () => fireEvent.click(view.getByTestId("folder-trust-restrict")));
    expect(calls()).toEqual(["trust:p1:untrusted"]);
    await act(async () => fireEvent.click(view.getByText("Start")));
    await act(async () => fireEvent.click(view.getByText("Cancel")));
    expect(view.getByTestId("went").textContent).toBe("stopped");
    expect(calls()).toEqual(["trust:p1:untrusted"]);
  });

  it("isn't asked again once answered, decided in the tool, or for a tool that asks nothing", async () => {
    for (const asked of [{ ...codex, choice: "trusted" }, { ...codex, choice: "in_tool" }, undefined]) {
      seams().__AIJ_TRUST__ = asked;
      const view = render(<Launcher />);
      await act(async () => fireEvent.click(view.getByText("Start")));
      expect(view.queryByTestId("folder-trust")).toBeNull();
      expect(view.getByTestId("went").textContent).toBe("went");
      cleanup();
    }
  });
});
