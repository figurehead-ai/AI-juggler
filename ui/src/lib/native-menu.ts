/**
 * AJ-E1537 — the right-click menu as the system's own (Tauri's menu
 * popup), so it looks and behaves like every other Mac menu. The same
 * registry actions as the in-app menu: a disabled item shows its reason
 * as a disabled line under it, and destructive items sit in their own
 * group at the end.
 *
 * AJ-E2106 — the same order and groups as the in-app menu (getting
 * there, working on it, stopping), the same one submenu level, and each
 * shortcut as the system shows keys. No icons here: Tauri's menu items
 * take a bitmap or one of AppKit's few named images, not a template
 * image, so a drawn icon would keep one colour on a highlighted line and
 * in dark menus; and a system menu line can't be red. The in-app menu
 * (⋯, Shift+F10) has both.
 */
import { menuEntries, type ResolvedAction } from "./actions";

/** "⌘⇧H" as the system's accelerator ("CmdOrCtrl+Shift+H"). */
export function accelerator(keys: string | undefined): string | undefined {
  if (!keys?.startsWith("⌘")) return undefined;
  const parts = ["CmdOrCtrl"];
  let rest = keys.slice(1);
  for (const [mark, name] of [["⌥", "Alt"], ["⌃", "Ctrl"], ["⇧", "Shift"]] as const) {
    if (rest.includes(mark)) {
      parts.push(name);
      rest = rest.replace(mark, "");
    }
  }
  return rest.length === 1 ? [...parts, rest.toUpperCase()].join("+") : undefined;
}

export async function popupNativeMenu(heading: string, actions: ResolvedAction[], at: { x: number; y: number }): Promise<void> {
  const { Menu, MenuItem, PredefinedMenuItem, Submenu } = await import("@tauri-apps/api/menu");
  const { LogicalPosition } = await import("@tauri-apps/api/dpi");
  const lines = async (action: ResolvedAction) => {
    const options = {
      text: action.label,
      enabled: !action.reason,
      action: () => {
        if (!action.reason) void action.run();
      },
    };
    const keys = accelerator(action.shortcut);
    // Keys the system can't read leave the line without them, never the
    // menu without the line.
    const line = keys ? await MenuItem.new({ ...options, accelerator: keys }).catch(() => MenuItem.new(options)) : await MenuItem.new(options);
    const made = [line];
    if (action.reason) made.push(await MenuItem.new({ text: `   ${action.reason}`, enabled: false }));
    return made;
  };
  const items = [];
  items.push(await MenuItem.new({ text: heading, enabled: false }));
  items.push(await PredefinedMenuItem.new({ item: "Separator" }));
  const entries = menuEntries(actions);
  for (const [index, entry] of entries.entries()) {
    if (index > 0 && entries[index - 1]!.group !== entry.group) items.push(await PredefinedMenuItem.new({ item: "Separator" }));
    if (entry.kind === "action") {
      items.push(...(await lines(entry.action)));
    } else {
      const inside = (await Promise.all(entry.actions.map(lines))).flat();
      items.push(await Submenu.new({ text: entry.label, items: inside }));
    }
  }
  const menu = await Menu.new({ items });
  await menu.popup(new LogicalPosition(at.x, at.y));
}
