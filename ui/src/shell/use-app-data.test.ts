/**
 * AJ-E805 (0.4.5, idle CPU): the shell's lists are read again on every
 * change of sessions or conversations (every few seconds with sessions at
 * work); an answer equal to the one shown keeps its identity, so nothing
 * that reads it draws again. It set new arrays every time.
 */
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { useAppData } from "./use-app-data";

afterEach(cleanup);

it("an equal answer keeps each list's identity", async () => {
  const { result } = renderHook(() => useAppData({ kind: "none" }, () => {}));
  await waitFor(() => expect(result.current.loaded).toBe(true));
  const { projects, sessions, accounts, profiles, freshness } = result.current;
  await act(async () => {
    await result.current.refresh();
  });
  expect(result.current.projects).toBe(projects);
  expect(result.current.sessions).toBe(sessions);
  expect(result.current.accounts).toBe(accounts);
  expect(result.current.profiles).toBe(profiles);
  expect(result.current.freshness).toBe(freshness);
});
