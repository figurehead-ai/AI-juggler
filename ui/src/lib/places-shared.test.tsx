/**
 * Walk A4: the places and the stored Apps are one read for every screen
 * that shows them (seven hooks each read the places, ~20 reads in a
 * window's first seconds).
 */
import { render, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { forgetSharedReads } from "./shared-read";

const calls = { places: 0, apps: 0 };
vi.mock("./ipc/places", async (actual) => ({
  ...(await actual<typeof import("./ipc/places")>()),
  placesList: async () => {
    calls.places += 1;
    return [];
  },
}));
vi.mock("./ipc/apps", async (actual) => ({
  ...(await actual<typeof import("./ipc/apps")>()),
  appsList: async () => {
    calls.apps += 1;
    return [];
  },
}));

afterEach(() => {
  forgetSharedReads();
  calls.places = 0;
  calls.apps = 0;
});

it("five screens showing the places and the Apps at once read each once", async () => {
  const { useKnownPlaces } = await import("./places");
  const { useApps } = await import("./apps");
  const folders: never[] = [];
  function Screen(): null {
    useKnownPlaces();
    useApps(folders);
    return null;
  }
  render(
    <>
      <Screen />
      <Screen />
      <Screen />
      <Screen />
      <Screen />
    </>,
  );
  await waitFor(() => expect(calls.places).toBeGreaterThan(0));
  await new Promise((done) => setTimeout(done, 50));
  expect(calls.places).toBe(1);
  expect(calls.apps).toBe(1);
});
