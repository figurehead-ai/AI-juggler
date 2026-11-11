/**
 * AJ-E3206 — a session's avatar is its Account's, in the Account's colour,
 * so its tooltip names that Account ("Claude Max"), not the folder the
 * session ran in ("Work Max"): the folder is an App, said beside it.
 */
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AccountAvatar } from "./Avatar";
import * as subs from "../lib/ipc/subscriptions";
import { loadSubscriptions, type SubscriptionInfo } from "../lib/subscriptions";
import { previewNamed } from "../preview/names";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const account = (key: string, name: string | null, ids: string[]): SubscriptionInfo =>
  // Named as the backend names it (the preview's stand-in for its rule).
  previewNamed([
    {
      key,
      name,
      tool: "claude-code",
      state: "signed_in",
      email: "o…@work.com",
      organisation: null,
      reason: null,
      profiles: ids.map((id) => ({ id, label: `Folder ${id}`, folder: `~/p/${id}`, plan: null, renewal_date: null, fee_usd: null })),
      plan: "max",
      renewal_date: null,
      fee_usd: null,
      differs: [],
      choices: [],
    },
  ])[0]!;

describe("an Account's avatar names the Account", () => {
  it("a folder's avatar says the Account it is signed in to, once hovered", async () => {
    vi.spyOn(subs, "subscriptionsList").mockResolvedValue([account("work", "Studio", ["w1"]), account("home", null, ["h1"])]);
    await loadSubscriptions(true);
    render(
      <>
        <AccountAvatar account={{ folder: "w1" }} label="Work Max" />
        <AccountAvatar account={{ folder: "h1" }} label="Home" />
        <AccountAvatar account={{ folder: "lost" }} label="Old folder" />
      </>,
    );
    const [work, home, lost] = screen.getAllByTestId("account-avatar");
    for (const avatar of [work!, home!, lost!]) fireEvent.pointerEnter(avatar);
    expect(work!.title).toBe("Studio");
    expect(home!.title).toBe("Claude Max");
    // A folder no Account lists keeps the words it was given.
    expect(lost!.title).toBe("Old folder");
  });
});
