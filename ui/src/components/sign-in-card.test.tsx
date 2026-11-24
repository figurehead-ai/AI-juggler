/**
 * AJ-E1553 / AJ-E1545 R8 — the sign-in card in each state, as a newcomer
 * sees it: the tool's page and its code (large, Copy), a code to paste
 * back, waiting, ✓ signed in, ended early (its last lines and Try again),
 * and a sign-in that can't be read (its terminal, one plain line). The
 * terminal under "Show details" stays collapsed until asked for.
 */
import { act, cleanup, fireEvent, render, renderHook, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SignInCard } from "./SignInCard";
import { SignInPanel } from "./SignInPanel";
import { RailSignIns } from "./RailSignIns";
import { refreshSignIns, resetSignInsForTests } from "../lib/sign-ins-store";
import { useLoginTerminals } from "../shell/use-login-terminals";
import type { SignInInfo } from "../lib/ipc/sign-ins";

vi.mock("./TerminalView", () => ({
  TerminalView: ({ id }: { id: string }) => <div data-testid={`terminal-${id}`} />,
}));

const writes: Array<[string, string]> = [];
vi.mock("../lib/ipc/terminal-io", () => ({
  terminalWrite: vi.fn(async (id: string, data: string) => {
    writes.push([id, data]);
  }),
  terminalKill: vi.fn(async () => undefined),
  terminalSpawnLogin: vi.fn(async (account: string) => ({ id: `login-next-${account}` })),
  onTerminalExit: vi.fn(() => () => undefined),
}));

const ID = "login-1";
const info = (over: Partial<SignInInfo>): SignInInfo => ({
  id: ID,
  account_id: "acct:openai:alice",
  tool: "codex",
  title: "Sign in · Codex",
  state: "running",
  page_host: null,
  code: null,
  pastes_code: false,
  last_lines: [],
  started_at: Date.UTC(2026, 10, 23, 9, 0, 0),
  ...over,
});

const card = (signIn: SignInInfo | null, onTryAgain = vi.fn()) =>
  render(<SignInCard signIn={signIn} terminalId={ID} tool="codex" title="Sign in · Codex" onTryAgain={onTryAgain} onDismiss={vi.fn()} />);

beforeEach(() => {
  writes.length = 0;
  window.__AIJ_SIGN_IN_PAGES_OPENED__ = [];
});
afterEach(() => {
  cleanup();
  delete window.__AIJ_SIGN_INS__;
  resetSignInsForTests();
});

describe("SignInCard", () => {
  it("Stop signing in ends a running sign-in, on the page and in the terminal alike (F29)", async () => {
    const { terminalKill } = await import("../lib/ipc/terminal-io");
    for (const running of [info({ page_host: "auth.openai.com" }), info({})]) {
      vi.mocked(terminalKill).mockClear();
      card(running);
      const stop = screen.getByRole("button", { name: "Stop signing in" });
      await act(async () => {
        fireEvent.click(stop);
      });
      expect(terminalKill).toHaveBeenCalledWith(ID);
      expect(screen.getByRole("button", { name: "Stopping…" })).toHaveProperty("disabled", true);
      cleanup();
    }
    // Ended or signed in: nothing to stop.
    card(info({ state: "ended" }));
    expect(screen.queryByTestId("sign-in-stop")).toBeNull();
  });

  it("shows the page as the one primary, the code large with Copy, and waits", async () => {
    const copy = vi.fn(async () => undefined);
    Object.defineProperty(navigator, "clipboard", { value: { writeText: copy }, configurable: true });
    card(info({ page_host: "auth.openai.com", code: "ABCD-1234" }));

    expect(screen.getByTestId("sign-in-card").dataset.state).toBe("page");
    const open = screen.getByRole("button", { name: "Open Codex's sign-in page" });
    fireEvent.click(open);
    expect(window.__AIJ_SIGN_IN_PAGES_OPENED__).toEqual([ID]);
    expect(screen.getByText("auth.openai.com")).toBeTruthy();

    const code = screen.getByTestId("sign-in-code");
    expect(code.textContent).toBe("ABCD-1234");
    expect(code.className).toContain("text-stat");
    expect(screen.getByText("Check the page shows this code, then approve.")).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Copy" }));
    });
    expect(copy).toHaveBeenCalledWith("ABCD-1234");
    expect(screen.getByRole("button", { name: "Copied" })).toBeTruthy();

    expect(screen.getByText("Waiting for you to approve…")).toBeTruthy();
    expect(screen.queryByTestId("sign-in-paste")).toBeNull();
  });

  it("keeps the terminal under Show details, collapsed until asked for", async () => {
    card(info({ page_host: "auth.openai.com", code: "ABCD-1234" }));
    const details = screen.getByTestId("sign-in-details");
    const toggle = screen.getByTestId("sign-in-details-toggle");
    expect(toggle.textContent).toContain("Show details");
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(details.dataset.open).toBeUndefined();
    const body = document.getElementById(toggle.getAttribute("aria-controls") ?? "");
    expect(body?.hidden).toBe(true);

    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect(body?.hidden).toBe(false);
    expect(await screen.findByTestId(`terminal-${ID}`)).toBeTruthy();
  });

  it("takes a code the page gives back and types it into the tool's sign-in", async () => {
    card(info({ tool: "claude-code", page_host: "claude.ai", pastes_code: true }));
    expect(screen.queryByTestId("sign-in-code")).toBeNull();
    const field = screen.getByTestId("sign-in-paste");
    const send = screen.getByRole("button", { name: "Send" });
    expect(send.hasAttribute("disabled")).toBe(true);
    fireEvent.change(field, { target: { value: "  pasted-code-42 " } });
    expect(send.hasAttribute("disabled")).toBe(false);
    fireEvent.click(send);
    expect(writes).toEqual([[ID, "pasted-code-42\r"]]);
    expect(field.getAttribute("value") ?? "").toBe("");
    expect(screen.getByText(/^Sent to /)).toBeTruthy();
    expect(screen.getByText("Waiting for you to approve…")).toBeTruthy();
  });

  it("says a page without a code plainly, still waiting", () => {
    card(info({ page_host: "auth.openai.com" }));
    expect(screen.getByText("Sign in on the page, then approve.")).toBeTruthy();
    expect(screen.getByText("Waiting for you to approve…")).toBeTruthy();
  });

  it("says ✓ signed in, with Done", () => {
    card(info({ state: "signed_in", page_host: "auth.openai.com", code: "ABCD-1234" }));
    expect(screen.getByTestId("sign-in-card").dataset.state).toBe("signed_in");
    expect(screen.getByRole("status").textContent).toBe("Codex is signed in");
    expect(screen.getByRole("button", { name: "Done" })).toBeTruthy();
    expect(screen.queryByTestId("sign-in-code")).toBeNull();
    expect(screen.queryByTestId("sign-in-terminal")).toBeNull();
  });

  it("shows an ended sign-in's last lines and Try again, never an empty terminal", () => {
    const tryAgain = vi.fn();
    card(info({ state: "ended", last_lines: ["Opening the browser…", "Error: the login timed out"] }), tryAgain);
    expect(screen.getByTestId("sign-in-card").dataset.state).toBe("ended");
    expect(screen.getByText("Sign-in ended before it finished")).toBeTruthy();
    expect(screen.getByTestId("sign-in-last-lines").textContent).toBe("Opening the browser…\nError: the login timed out");
    expect(screen.queryByTestId("sign-in-terminal")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(tryAgain).toHaveBeenCalledOnce();
    expect(screen.getByRole("button", { name: "Dismiss" })).toBeTruthy();
  });

  it("says so when an ended sign-in printed nothing", () => {
    card(info({ state: "ended" }));
    expect(screen.getByText("Codex said nothing before it ended.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Try again" })).toBeTruthy();
  });

  it("shows the terminal with one plain line when nothing can be read", async () => {
    card(info({}));
    expect(screen.getByTestId("sign-in-card").dataset.state).toBe("terminal");
    expect(screen.getByText("Codex's sign-in runs below: follow its steps there. AI Juggler notices when you're done.")).toBeTruthy();
    expect(screen.getByTestId("sign-in-terminal")).toBeTruthy();
    expect(await screen.findByTestId(`terminal-${ID}`)).toBeTruthy();
    expect(screen.queryByRole("button", { name: /sign-in page/ })).toBeNull();
    expect(screen.queryByTestId("sign-in-details")).toBeNull();
  });

  it("shows a sign-in not listed yet as its terminal (starting)", () => {
    card(null);
    expect(screen.getByTestId("sign-in-card").dataset.state).toBe("terminal");
  });
});

describe("SignInPanel", () => {
  const terminal = { id: ID, title: "Sign in · Codex", ended: false, account: "acct:openai:alice" };

  it("reads the window's sign-in list: page and code, Terminal chip, Stop while it runs", async () => {
    window.__AIJ_SIGN_INS__ = [info({ page_host: "auth.openai.com", code: "ABCD-1234" })];
    render(<SignInPanel terminal={terminal} accounts={[]} />);
    expect(await screen.findByTestId("sign-in-code")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Sign in · Codex" })).toBeTruthy();
    expect(screen.getByTestId("sign-in-terminal-chip").textContent).toBe("Terminal");
    expect(screen.getByRole("button", { name: "Stop" })).toBeTruthy();
    expect(screen.getByTestId("sign-in-details-toggle").getAttribute("aria-expanded")).toBe("false");
  });

  it("says ✓ signed in, with no Stop, and Done takes it off the list", async () => {
    window.__AIJ_SIGN_INS__ = [info({ state: "signed_in" })];
    const closed = vi.fn();
    render(<SignInPanel terminal={terminal} accounts={[]} onClosed={closed} />);
    expect(await screen.findByText("Codex is signed in")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Done" }));
    await waitFor(() => expect(closed).toHaveBeenCalledOnce());
    expect(window.__AIJ_SIGN_INS__).toEqual([]);
  });

  it("Try again starts a new sign-in for the same App and opens it", async () => {
    window.__AIJ_SIGN_INS__ = [info({ state: "ended", last_lines: ["Error: cancelled"] })];
    const opened = vi.fn();
    render(<SignInPanel terminal={{ ...terminal, ended: true }} accounts={[]} openLoginTerminal={opened} />);
    fireEvent.click(await screen.findByRole("button", { name: "Try again" }));
    await waitFor(() => expect(opened).toHaveBeenCalledWith("login-next-acct:openai:alice", "Sign in · Codex", { account: "acct:openai:alice" }));
  });
});

describe("the window's sign-ins (R1)", () => {
  it("a sign-in that finishes leaves the rail by itself and flips the lists once; an early end stays", async () => {
    window.__AIJ_SIGN_INS__ = [info({}), info({ id: "login-2", title: "Sign in · Grok", tool: "grok-build", state: "ended" })];
    const signedIn = vi.fn();
    const { result } = renderHook(() => useLoginTerminals(signedIn));
    await waitFor(() => expect(result.current.loginTerminals).toHaveLength(2));
    const rail = (terminals = result.current.loginTerminals) =>
      render(<RailSignIns terminals={terminals} selectedId={null} onSelect={vi.fn()} />);
    const before = rail();
    expect(screen.getByText("Sign in · Codex")).toBeTruthy();
    expect(screen.getByText("Didn't finish")).toBeTruthy();
    before.unmount();

    window.__AIJ_SIGN_INS__ = [info({ state: "signed_in" }), info({ id: "login-2", title: "Sign in · Grok", tool: "grok-build", state: "ended" })];
    await act(() => refreshSignIns());
    expect(signedIn).toHaveBeenCalledOnce();
    rail();
    expect(screen.queryByText("Sign in · Codex")).toBeNull();
    expect(screen.getByText("Sign in · Grok")).toBeTruthy();
    // Listed again unchanged: not flipped twice.
    await act(() => refreshSignIns());
    expect(signedIn).toHaveBeenCalledOnce();
  });

  it("one opened here shows at once, and goes once the backend's list drops it", async () => {
    window.__AIJ_SIGN_INS__ = [];
    const { result } = renderHook(() => useLoginTerminals(vi.fn()));
    act(() => result.current.opened({ id: "login-9", title: "Sign in · Codex", ended: false, account: "acct:openai:alice" }));
    expect(result.current.loginTerminals.map((one) => one.id)).toEqual(["login-9"]);
    window.__AIJ_SIGN_INS__ = [info({ id: "login-9" })];
    await act(() => refreshSignIns());
    expect(result.current.loginTerminals.map((one) => one.id)).toEqual(["login-9"]);
    // Dismissed: the backend's list is the truth, the local copy is gone.
    window.__AIJ_SIGN_INS__ = [info({ id: "login-other" })];
    await act(() => refreshSignIns());
    expect(result.current.loginTerminals.map((one) => one.id)).toEqual(["login-other"]);
  });
});
