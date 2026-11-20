/**
 * SK-8 (the soak's P3): a pool's Tasks opened on an empty body, only its
 * title and "Back to the canvas", for ~7 s on a busy Mac, then "No tasks
 * yet…". Its code loads on first open: a skeleton shows meanwhile, at
 * once, as its list does while read (instant and paged).
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { PoolPanel } from "./PoolPanel";
import LazySource from "../../lib/lazy.tsx?raw";

afterEach(cleanup);

describe("a pool's Tasks and Messages open on a skeleton, never an empty body", () => {
  it("Tasks shows its skeleton in the first render, before its code is here, then its list", async () => {
    render(<PoolPanel panel="tasks" poolId="pool-team" taskKey={0} sessions={[]} projects={[]} members={[]} now={0} onClose={() => undefined} />);
    expect(screen.getByRole("status", { name: "Loading the tasks" })).toBeTruthy();
    expect(await screen.findByTestId("pool-tasks")).toBeTruthy();
  });

  it("Messages too", () => {
    render(<PoolPanel panel="messages" poolId="pool-team" taskKey={0} sessions={[]} projects={[]} members={[]} now={0} onClose={() => undefined} />);
    expect(screen.getByRole("status", { name: "Loading the messages" })).toBeTruthy();
  });

  it("a lazy part's placeholder is drawn by the one loader", () => {
    expect(LazySource).toContain("<Pending>{placeholder?.()}</Pending>");
  });
});
