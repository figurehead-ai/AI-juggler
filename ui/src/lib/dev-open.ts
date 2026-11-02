/**
 * QA only (docs/qa/DEV-BRIDGE.md, "What the bridge can't show"): a Dev
 * window opened at `?open=<conversation id>` opens that session's page,
 * as a click on its rail row would, so a walk can watch a conversation
 * (message text, which the read-only bridge never serves) in the window
 * itself. Loaded only in a development build; a release build has none.
 */

/** How long the row is looked for (the first scan can take a while). */
const LOOK_MS = 60_000;

/** Clicks the rail row of conversation `id` once it's listed. */
export function openWhenListed(id: string, every = 500): void {
  if (!/^[A-Za-z0-9_-]{1,80}$/.test(id)) return;
  const until = Date.now() + LOOK_MS;
  const look = () => {
    const row = document.querySelector<HTMLElement>(`[data-conversation-id="${id}"]`);
    if (row) row.click();
    else if (Date.now() < until) setTimeout(look, every);
  };
  look();
}
