/**
 * AJ-E2108's budget: a session page shows its head (what it is, where it
 * runs) in under 150 ms. Another app's session, from the preview's own
 * data: time from the render to its head, the median of five, in jsdom
 * (slower than the app's webview).
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { ConversationPage } from "./ConversationPage";
import { ConversationsProvider } from "../lib/conversations";
import { ActionHostProvider } from "./ItemMenu";
import { placesConversations } from "../preview/places";
import { setSeam, clearSeams } from "../test/fixtures";

afterEach(() => {
  cleanup();
  clearSeams("__AIJ_FIXTURE__");
});

it("shows another app's session's head inside 150 ms", async () => {
  const now = Date.now();
  const conversations = placesConversations(now);
  const one = conversations[0]!;
  setSeam("__AIJ_FIXTURE__", { conversations });
  const times: number[] = [];
  for (let run = 0; run < 5; run += 1) {
    const started = performance.now();
    render(
      <ConversationsProvider>
        <ActionHostProvider host={{}}>
          <ConversationPage accountId={one.account_id} id={one.id} now={now} />
        </ActionHostProvider>
      </ConversationsProvider>,
    );
    await screen.findByTestId("conversation-page", undefined, { interval: 1 });
    times.push(performance.now() - started);
    cleanup();
  }
  times.sort((a, b) => a - b);
  console.info(`[perf] another app's session page, head shown: ${times.map((t) => t.toFixed(1)).join(", ")} ms`);
  expect(times[2]).toBeLessThan(150);
});
