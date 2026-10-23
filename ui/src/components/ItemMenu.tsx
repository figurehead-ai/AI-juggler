/**
 * AJ-E1537 — one menu, three ways in. A right-click opens the system's
 * own menu in the app (Tauri's menu popup), a row's or a header's ⋯
 * button and Shift+F10 (or the menu key) open the in-app menu, and every
 * one of them lists the same actions from the registry (`lib/actions`),
 * in the same order. Disabled items say why. Destructive ones come last,
 * in their own group.
 */
import { createContext, useContext, useMemo, useState, type JSX, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";
import { resolveActions, type ActionHost, type ItemKind, type ItemOf, type ResolvedAction } from "../lib/actions";
import { hasLiveBackend } from "../lib/bridge";
import { popupNativeMenu } from "../lib/native-menu";
import { MenuItem, MenuSeparator, RowMenu, menuPoint } from "./RowMenu";

const HostContext = createContext<ActionHost>({});

/** Gives the actions below it the capabilities this screen owns, on top
 * of those from further up (App's). */
export function ActionHostProvider({ host, children }: { host: ActionHost; children: ReactNode }): JSX.Element {
  const parent = useContext(HostContext);
  const merged = useMemo(() => ({ ...parent, ...host }), [parent, host]);
  return <HostContext.Provider value={merged}>{children}</HostContext.Provider>;
}

export function useActionHost(): ActionHost {
  return useContext(HostContext);
}

/** A right-click opens the native menu when the app runs for real; in the
 * preview, tests and the Dev bridge, the in-app one. */
function nativeMenus(): boolean {
  return hasLiveBackend() && !(window as unknown as { __AIJ_IN_APP_MENUS__?: boolean }).__AIJ_IN_APP_MENUS__;
}

export interface ItemMenu {
  /** For the row or header: right-click and Shift+F10 open the menu. */
  triggerProps: {
    onContextMenu: (event: MouseEvent<HTMLElement>) => void;
    onKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
  };
  /** The ⋯ button that opens it: `shown` keeps it visible (a card's
   * header); otherwise it appears on hover or focus (a row's). */
  button: (options: { label: string; className?: string; shown?: boolean; testId?: string }) => JSX.Element;
  /** The open menu (render it anywhere: it goes on the page itself). */
  element: JSX.Element | null;
  /** The items, as every trigger shows them. */
  actions: ResolvedAction[];
}

/** The menu of `item`, a `kind`. `heading` names it on top; `local` adds
 * what only this row or card can do (its own sheets and toggles). */
export function useItemMenu<K extends ItemKind>(kind: K, item: ItemOf[K], heading: string, local?: ActionHost): ItemMenu {
  const shared = useActionHost();
  const host = local ? { ...shared, ...local } : shared;
  const [at, setAt] = useState<{ x: number; y: number } | null>(null);
  const actions = resolveActions(kind, item, host);
  const open = (point: { x: number; y: number }, native: boolean) => {
    if (actions.length === 0) return;
    if (native) void popupNativeMenu(heading, actions, point);
    else setAt(point);
  };
  const run = (action: ResolvedAction) => () => {
    setAt(null);
    if (!action.reason) void action.run();
  };
  let element: JSX.Element | null = null;
  if (at) {
    element = (
      <RowMenu heading={heading} ariaLabel={`Actions for ${heading}`} at={at} onClose={() => setAt(null)}>
        {actions.map((action, index) => (
          <ItemMenuLine key={action.id} action={action} after={actions[index - 1]} onRun={run(action)} />
        ))}
      </RowMenu>
    );
  }
  return {
    actions,
    element,
    triggerProps: {
      onContextMenu: (event) => {
        if (actions.length === 0) return;
        event.preventDefault();
        event.stopPropagation();
        open({ x: event.clientX, y: event.clientY }, nativeMenus());
      },
      onKeyDown: (event) => {
        if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
          event.preventDefault();
          event.stopPropagation();
          const box = event.currentTarget.getBoundingClientRect();
          open({ x: box.left + 8, y: box.bottom + 4 }, false);
        }
      },
    },
    button: ({ label, className = "", shown = false, testId }) => (
      <button
        type="button"
        aria-label={label}
        aria-haspopup="menu"
        aria-expanded={at !== null}
        data-testid={testId}
        disabled={actions.length === 0}
        onClick={(event) => open(menuPoint(event), false)}
        className={
          "z-10 flex size-6 shrink-0 items-center justify-center rounded outline-none transition-colors focus-visible:ring-1 focus-visible:ring-accent/60 disabled:hidden " +
          (shown
            ? "text-[12px] text-muted hover:bg-surface-2 hover:text-ink "
            : "text-[10px] text-faint opacity-0 transition-opacity hover:text-ink focus-visible:opacity-100 group-hover:opacity-100 ") +
          className
        }
      >
        ⋯
      </button>
    ),
  };
}

function ItemMenuLine({ action, after, onRun }: { action: ResolvedAction; after?: ResolvedAction; onRun: () => void }): JSX.Element {
  return (
    <>
      {after && after.group !== action.group && <MenuSeparator />}
      {/* Its name is its label: the keys are decoration, and the reason
          sits under it (and in its tooltip), outside the item. */}
      <MenuItem testId={`action-${action.id}`} onClick={onRun} disabled={!!action.reason} title={action.reason} danger={action.destructive}>
        <span className="flex items-baseline justify-between gap-3">
          <span data-label>{action.label}</span>
          {action.shortcut && (
            <span aria-hidden="true" className="font-mono text-[10px] text-faint">
              {action.shortcut}
            </span>
          )}
        </span>
      </MenuItem>
      {action.reason && (
        <span aria-hidden="true" className="-mt-0.5 block px-2 pb-1 text-[10px] leading-snug text-faint">
          {action.reason}
        </span>
      )}
    </>
  );
}
