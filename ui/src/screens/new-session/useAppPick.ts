/**
 * F-rc3f-1 (the 0.4.6 rc3f walk; persona Priya: work started on the wrong
 * client's Account) — New session's App across a folder change, never
 * moved unseen:
 *
 * - The App the person picked (Change app, an offer, Undo, the door that
 *   opened the sheet, the folder they just bound with "Always start …")
 *   stays. In a folder whose own App is another (its default, its only
 *   Account, the one used there), that one is named and offered (`ask`),
 *   never taken.
 * - A suggested App is suggested again for the new folder; when it moves,
 *   the card says from which (`change`), with Undo.
 * - `acted`: the person did something that may move the App; from then
 *   on the live region says the App and its Account.
 */
import { useState } from "react";
import { FOLDER_OWN, type AppChoice } from "../../lib/new-session-choice";
import type { LaunchRecipeInfo } from "../../types/generated/LaunchRecipeInfo";

export interface AppPick {
  /** The App (its folder id) the person picked; `null`: New session suggests. */
  chosen: string | null;
  acted: boolean;
  /** The person picks App `id`: theirs, here and in the next folder. The
   * App shown picked again counts (F-rc3g-5): what the card said a folder
   * change did to it goes. */
  pick: (id: string) => void;
  /** The folder changes; `shown`: the App the card showed before (`null`
   * when it stays the same folder). */
  moving: (shown: AppChoice | null) => void;
  /** The person bound the folder to App `id` ("Always start …"), or
   * cleared its default (`null`, which keeps any App picked). */
  bind: (id: string | null) => void;
  /** What the card says about it: the App a suggestion moved from, and
   * the folder's own App when the person's pick was carried here. */
  told: (app: AppChoice | null, suggested: AppChoice | null) => { change: LaunchRecipeInfo | null; ask: AppChoice | null };
}

export function useAppPick(initial: string | null): AppPick {
  const [chosen, setChosen] = useState<string | null>(initial);
  const [moved, setMoved] = useState<LaunchRecipeInfo | null>(null);
  const [carried, setCarried] = useState(false);
  const [acted, setActed] = useState(false);
  const [bound, setBound] = useState<string | null>(null);
  return {
    chosen,
    acted,
    pick: (id) => {
      setActed(true);
      setChosen(id);
      setMoved(null);
      setCarried(false);
    },
    moving: (shown) => {
      setActed(true);
      if (!shown) return;
      const mine = chosen ?? (bound === shown.recipe.account_id ? bound : null);
      if (mine !== null) {
        setChosen(mine);
        setCarried(true);
        setMoved(null);
      } else {
        setMoved(shown.recipe);
        setCarried(false);
      }
    },
    bind: (id) => {
      setActed(true);
      setBound(id);
      if (id === null) return;
      setChosen(null);
      setMoved(null);
      setCarried(false);
    },
    told: (app, suggested) => ({
      change: moved && chosen === null && app && app.recipe.account_id !== moved.account_id ? moved : null,
      ask: carried && chosen !== null && app && suggested && suggested.recipe.account_id !== app.recipe.account_id && FOLDER_OWN.has(suggested.why) ? suggested : null,
    }),
  };
}
