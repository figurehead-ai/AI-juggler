/**
 * ADR-0031 (D-93): a Move to account offer for the gallery and the
 * dialogs' layout check (`__AIJ_MOVE_OFFER__`): a conversation with four
 * more of its family, two restarts, a kept-aside note, the project-notes
 * choice and two kept tool-output folders. Generic names only.
 */
import type { MoveOfferInfo } from "../lib/ipc/switch-move";

export const EXAMPLE_MOVE_OFFER: MoveOfferInfo = {
  applies: true,
  blocks: [],
  members: [
    { title: "Fix the checkout flow", archived: false, side: "moves", folder_missing: false },
    { title: "Checkout tests", archived: false, side: "moves", folder_missing: true },
    { title: "Payment webhooks", archived: false, side: "moves", folder_missing: true },
    { title: "Old spike", archived: true, side: "moves", folder_missing: true },
    { title: "Refund rules", archived: false, side: "relinked", folder_missing: true },
  ],
  restarts: ["Claude app · Work", "Claude app · Personal"],
  notes: ["the target had its own different version of “Checkout tests”; it is kept aside, never joined to this one, and Undo brings it back"],
  notes_conflict: "CLAUDE.md",
  kept_outputs: 2,
  bytes: 1_200_000_000,
  app: true,
};
