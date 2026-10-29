/**
 * D-49 test kit: the backend works Needs you out; a test that renders Home
 * or the rail alone hands its fixture to the preview's stand-in for the
 * backend (preview/needs.tsx, the same rules), as the backend's tick
 * would, so the views read it the way they read the backend.
 */
import { resetNoticeStandIn, standInNeeds } from "../lib/ipc/notices";
import { forgetPaged } from "../lib/paged";
import { resetNotices } from "../lib/notices-store";
import { previewNeedsOf, type PreviewInputs } from "../preview/needs";
import type { InstanceInfo } from "../lib/bridge";
import type { ReactElement } from "react";
import { render, type RenderResult } from "@testing-library/react";

/** Needs you, as the backend would have it for this fixture. */
export function needsFor(inputs: PreviewInputs): void {
  resetNoticeStandIn();
  resetNotices();
  forgetPaged();
  standInNeeds(previewNeedsOf(inputs));
}

/** Renders `ui` once the fixture's Needs you is in place. */
export function renderWithNeeds(inputs: PreviewInputs, ui: ReactElement): RenderResult {
  needsFor(inputs);
  return render(ui);
}

/** …for rows of sessions alone. */
export function needsOfRows(rows: InstanceInfo[]): void {
  needsFor({ sessions: rows });
}
