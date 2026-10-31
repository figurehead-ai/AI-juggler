/**
 * 0.3.2 — no web-view menu where AI Juggler has none of its own (the
 * maintainer's 0.3.0 walk: a right-click on the rail's group header showed
 * the browser's own menu, with Reload). A right-click a row's own menu
 * took (ItemMenu, the pool canvas) is left to it. The web view's menu
 * stays where it helps: in a text field or the terminal (Paste), and on
 * selected text (Copy). In a Dev build, ⌥-right-click still opens it, for
 * inspecting the page.
 */

/** Where the web view's own menu stays. */
function keepsNativeMenu(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  if (target.closest("input, textarea, [contenteditable=''], [contenteditable='true'], .xterm")) return true;
  const selection = window.getSelection?.();
  return selection != null && !selection.isCollapsed && selection.toString().trim() !== "";
}

/** What a right-click does: `native` (the web view's menu), or `none`. */
export function contextMenuFor(event: Pick<MouseEvent, "defaultPrevented" | "altKey" | "target">, devBuild: boolean): "native" | "none" | "taken" {
  if (event.defaultPrevented) return "taken";
  if (devBuild && event.altKey) return "native";
  return keepsNativeMenu(event.target) ? "native" : "none";
}

/** Installs the rule once for the window; `devBuild` is asked each time
 * (the edition is known only once the backend answers). Returns its undo. */
export function guardContextMenu(devBuild: () => boolean): () => void {
  const on = (event: MouseEvent) => {
    if (contextMenuFor(event, devBuild()) === "none") event.preventDefault();
  };
  window.addEventListener("contextmenu", on);
  return () => window.removeEventListener("contextmenu", on);
}
