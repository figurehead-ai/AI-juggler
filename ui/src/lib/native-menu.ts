/**
 * AJ-E1537 — the right-click menu as the system's own (Tauri's menu
 * popup), so it looks and behaves like every other Mac menu. The same
 * registry actions as the in-app menu: a disabled item shows its reason
 * as a disabled line under it, and destructive items sit in their own
 * group at the end.
 */
import type { ResolvedAction } from "./actions";

export async function popupNativeMenu(heading: string, actions: ResolvedAction[], at: { x: number; y: number }): Promise<void> {
  const { Menu, MenuItem, PredefinedMenuItem } = await import("@tauri-apps/api/menu");
  const { LogicalPosition } = await import("@tauri-apps/api/dpi");
  const items = [];
  items.push(await MenuItem.new({ text: heading, enabled: false }));
  items.push(await PredefinedMenuItem.new({ item: "Separator" }));
  let group = actions[0]?.group;
  for (const action of actions) {
    if (action.group !== group) {
      items.push(await PredefinedMenuItem.new({ item: "Separator" }));
      group = action.group;
    }
    items.push(
      await MenuItem.new({
        text: action.label,
        enabled: !action.reason,
        action: () => {
          if (!action.reason) void action.run();
        },
      }),
    );
    if (action.reason) items.push(await MenuItem.new({ text: `   ${action.reason}`, enabled: false }));
  }
  const menu = await Menu.new({ items });
  await menu.popup(new LogicalPosition(at.x, at.y));
}
