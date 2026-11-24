/**
 * AJ-E2023 Req.2 — the signed file's notices ride the app-wide band:
 * shown through the shared D-126 Banner (calm words, D-117),
 * dismissible only when the file says, an empty file silent.
 */
import { render, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { NotificationRegion } from "./NotificationRegion";

describe("AJ-E2023 remote notices in the band", () => {
  it("shows the signed file's notice as a calm banner with its link words", async () => {
    Reflect.set(window, "__AIJ_REMOTE__", {
      notices: [
        {
          id: "n1",
          level: "info",
          title: "Claude Code 2.2 needs a new switch route",
          body: "Until it's fixed, one route is hidden.",
          link: "https://figurehead.ai/notes/claude-22",
          link_label: "What happened",
          dismissable: true,
        },
      ],
    });
    const mounted = render(<NotificationRegion onOpenRemote={undefined} />);
    const banner = await waitFor(() => {
      const found = mounted.container.querySelector("[data-testid='remote-notice']");
      expect(found).not.toBeNull();
      return found!;
    });
    expect(banner.textContent).toContain("Claude Code 2.2");
    expect(banner.textContent).toContain("What happened");
    // D-117: calm words — no exclamation, no all-caps shout.
    expect(banner.textContent).not.toMatch(/!|[A-Z]{6,}/);
    Reflect.deleteProperty(window, "__AIJ_REMOTE__");
  });

  it("an empty file shows nothing", async () => {
    Reflect.set(window, "__AIJ_REMOTE__", { notices: [] });
    const mounted = render(<NotificationRegion onOpenRemote={undefined} />);
    await waitFor(() =>
      expect(mounted.container.querySelector("[data-testid='remote-notice']")).toBeNull(),
    );
    Reflect.deleteProperty(window, "__AIJ_REMOTE__");
  });
});
