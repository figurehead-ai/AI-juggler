/**
 * AJ-E2327 — the rail's Add lets its own change through the hold and
 * brings the added App's line into view once a read of the places draws
 * it.
 */
import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { useRef, type JSX } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useAddOffer } from "./rail-add";
import { LINE_KEY } from "./rail-hold";
import { knownKey } from "../lib/rail-apps";
import type { KnownPlaceInfo } from "../lib/ipc/places";
import type { PlaceOfferInfo } from "../types/generated/PlaceOfferInfo";
import type { PlaceAddedInfo } from "../types/generated/PlaceAddedInfo";

afterEach(cleanup);

const ID = "app:claude-app:/Users/you/Library/Application Support/ClaudeProfiles/team-a";
const offer: PlaceOfferInfo = { id: ID, label: "Claude app · team-a", running: true };
const added: KnownPlaceInfo = {
  id: ID, app_id: "claude-app", app: "Claude", instance: "team-a", profile: "team-a", label: "Claude · team-a", app_pid: 100, lost: false,
  keep_alive: false, paused: null, account_id: "claude-code-1", seen_ms: 1, cannot_start: null, recent: [], found: false,
};

function Rail({ places, own, answer }: { places: KnownPlaceInfo[]; own: () => void; answer: () => Promise<PlaceAddedInfo | null> }): JSX.Element {
  const root = useRef<HTMLDivElement | null>(null);
  const add = useAddOffer(root, own, answer, places);
  return (
    <div ref={root}>
      <button type="button" onClick={() => add(offer)}>
        Add
      </button>
      {places.map((place) => (
        <div key={place.id} {...{ [LINE_KEY]: `header:${knownKey(place)}` }} data-testid="added-line" />
      ))}
    </div>
  );
}

describe("AJ-E2327 the rail's Add", () => {
  it("lets its change through and brings the added App into view", async () => {
    const own = vi.fn();
    const seen: Element[] = [];
    HTMLElement.prototype.scrollIntoView = function (this: HTMLElement) {
      seen.push(this);
    };
    const answer = () => Promise.resolve({ id: ID, folder: { tool: "Claude Code", name: "client-a", path: null, by: "running", why: null } });
    const view = render(<Rail places={[]} own={own} answer={answer} />);
    await act(async () => {
      fireEvent.click(view.getByText("Add"));
      await Promise.resolve();
    });
    expect(own).toHaveBeenCalled();
    view.rerender(<Rail places={[added]} own={own} answer={answer} />);
    expect(seen).toEqual([view.getByTestId("added-line")]);
    Reflect.deleteProperty(HTMLElement.prototype, "scrollIntoView");
  });
});
