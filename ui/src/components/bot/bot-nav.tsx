/**
 * D-136 — where Bot's links go, by id (actions target by id): a row opens
 * its record's page; "Go to it" brings a session forward in its app. The
 * shell provides these (App.tsx); a test or the gallery may not.
 */
import { createContext, useContext } from "react";
import type { BotRowInfo } from "../../types/generated/BotRowInfo";
import type { BotShortcut } from "../../types/generated/BotShortcut";

export interface BotNav {
  /** The record's page. */
  openRow: (row: BotRowInfo) => void;
  /** A live session brought forward in its app (`null`: not live). */
  goTo: ((row: BotRowInfo) => void) | null;
  /** The screen a shortcut names. */
  openShortcut: (shortcut: BotShortcut) => void;
  /** Settings › AI (or one of its panes). */
  openSettings: (pane?: "ai-models" | "ai") => void;
}

const NONE: BotNav = {
  openRow: () => undefined,
  goTo: null,
  openShortcut: () => undefined,
  openSettings: () => undefined,
};

export const BotNavContext = createContext<BotNav>(NONE);

export function useBotNav(): BotNav {
  return useContext(BotNavContext);
}
