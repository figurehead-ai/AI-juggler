/**
 * D-126, the rc3 critique of the wizard's gate renders (98-pool-setup at
 * 1280×800): words too long for their place, and a preview that scrolled
 * its head away. Two CLI Apps of one tool ("Claude Code CLI · Work Max",
 * "Claude Code CLI · Personal") are the case it found.
 */
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { MOCK_ACCOUNTS, MOCK_PROJECTS, MOCK_SESSIONS } from "../../preview/fixtures";
import { resetPreviewPools } from "../../lib/ipc/pools";
import { resetPreviewPoolModel } from "../../lib/ipc/pool-model";
import { refreshPools, resetPools } from "../../lib/pools";
import { NewPool } from "./NewPool";

beforeEach(async () => {
  localStorage.clear();
  resetPreviewPools();
  resetPreviewPoolModel();
  resetPools();
  await refreshPools();
});
afterEach(cleanup);

/** New pool on Who's in with both CLI Apps ticked, its lead the first's
 * next session. */
async function twoCliApps(): Promise<void> {
  render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={MOCK_ACCOUNTS} project="proj-ai-juggler" onClose={() => undefined} />);
  for (let n = 0; n < 2; n += 1) fireEvent.click(screen.getByTestId("new-pool-next"));
  const cards = await screen.findAllByTestId("new-pool-candidate");
  for (const ref of ["app:cli:claude-work", "app:cli:claude-personal"]) fireEvent.click(cards.find((card) => card.dataset.ref === ref)!.querySelector("input")!);
  fireEvent.click(screen.getByTestId("new-pool-next"));
  fireEvent.click(screen.getByRole("radio", { name: /^Claude Code CLI · Work Max's next session/ }));
}

const stepValue = (step: string) => screen.getAllByTestId("flow-step").find((one) => one.dataset.step === step)!.querySelector('[id$="-more"]')!.textContent;

describe("the rc3 critique", () => {
  it("#1: the steps say an App by its app word, or its profile where two share one", async () => {
    await twoCliApps();
    expect(stepValue("who")).toBe("Work Max, Personal, everyone in ai-juggler, done");
    fireEvent.click(screen.getByTestId("new-pool-next"));
    expect(stepValue("lead")).toBe("Work Max's next session, done");
  });

  it("#2: three across, everyone's card is \"Everyone / in ai-juggler\", as the mock draws it", async () => {
    render(<NewPool sessions={MOCK_SESSIONS} projects={MOCK_PROJECTS} accounts={MOCK_ACCOUNTS} project="proj-ai-juggler" onClose={() => undefined} />);
    for (let n = 0; n < 2; n += 1) fireEvent.click(screen.getByTestId("new-pool-next"));
    for (const card of await screen.findAllByTestId("new-pool-candidate")) fireEvent.click(card.querySelector("input")!);
    expect(screen.getAllByTestId("new-pool-preview-node").map((node) => node.textContent)).toEqual(["Work MaxMember", "PersonalMember", "Everyonein ai-juggler"]);
  });

  it("#2: the preview's cards go by the same short names, and a long one wraps before it's cut", async () => {
    await twoCliApps();
    const names = screen.getAllByTestId("new-pool-preview-node-name");
    expect(names.map((name) => name.textContent)).toEqual(["Work Max", "Personal", "Everyone in ai-juggler"]);
    for (const name of names) {
      expect(name.className).toContain("line-clamp-2");
      expect(name.className).not.toContain("truncate");
    }
  });

  it("#3: the preview is never taller than the window less its head and foot; its rows scroll inside it", async () => {
    await twoCliApps();
    await waitFor(() => expect(screen.getByTestId("new-pool-preview")).toBeTruthy());
    expect(screen.getByTestId("new-pool-preview").className).toMatch(/max-h-\[calc\(100dvh-\d+px\)\]/);
    expect(screen.getByTestId("new-pool-preview-scroll").className).toContain("overflow-y-auto");
    // It scrolls on a short window, so the keyboard reaches it (axe scrollable-region-focusable).
    const words = screen.getByRole("region", { name: "The team in words" });
    expect(words.getAttribute("tabindex")).toBe("0");
    // In the layout's sticky column, beside the form.
    expect(screen.getByTestId("flow-preview").className).toContain("sticky");
  });
});
