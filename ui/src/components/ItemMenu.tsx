/**
 * AJ-E1537 — one menu, three ways in. A right-click, a row's or a
 * header's ⋯ button, and Shift+F10 (or the menu key) all open the same
 * in-app menu, listing the same actions from the registry
 * (`lib/actions`), in the same order. Disabled items say why. Destructive
 * ones come last, in their own group.
 *
 * AJ-E2106 — the right-click opens this menu too, not the system's: only
 * this one has the icons, the red stopping group and the greyed reasons
 * (the approved design). It opens at the pointer and stays inside the
 * window (`RowMenu` measures itself). `lib/native-menu.ts` stays for the
 * app's menu bar.
 */
import { Fragment, createContext, useContext, useId, useMemo, useState, type JSX, type KeyboardEvent, type MouseEvent, type ReactNode } from "react";
import { menuEntries, resolveActions, type ActionHost, type ActionIconName, type ItemKind, type ItemOf, type ResolvedAction } from "../lib/actions";
import { ActionIcon } from "./ActionIcon";
import { MenuItem, MenuSeparator, MenuSubmenu, RowMenu, menuPoint } from "./RowMenu";

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
  const open = (point: { x: number; y: number }) => {
    if (actions.length > 0) setAt(point);
  };
  /** Under the row, for a menu opened from the keyboard. */
  const below = (element: HTMLElement) => {
    const box = element.getBoundingClientRect();
    return { x: box.left + 8, y: box.bottom + 4 };
  };
  const run = (action: ResolvedAction) => () => {
    setAt(null);
    if (!action.reason) void action.run();
  };
  let element: JSX.Element | null = null;
  if (at) {
    element = (
      <RowMenu heading={heading} ariaLabel={`Actions for ${heading}`} at={at} onClose={() => setAt(null)}>
        <ItemMenuLines actions={actions} run={run} />
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
        // At the pointer; a contextmenu the keyboard sent has no pointer
        // position, so it opens under the row.
        const fromKeys = event.button !== 2 && event.clientX === 0 && event.clientY === 0;
        open(fromKeys ? below(event.currentTarget) : { x: event.clientX, y: event.clientY });
      },
      onKeyDown: (event) => {
        if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
          event.preventDefault();
          event.stopPropagation();
          open(below(event.currentTarget));
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
        onClick={(event) => open(menuPoint(event))}
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

/** AJ-E2106: an icon, the label, the keys; a submenu's line ends in ›. */
function Line({ icon, label, keys, more }: { icon: ActionIconName; label: string; keys?: string; more?: boolean }): JSX.Element {
  return (
    <span className="flex items-center gap-2">
      <ActionIcon name={icon} />
      <span data-label className="min-w-0 flex-1 truncate">
        {label}
      </span>
      {keys && (
        <span aria-hidden="true" className="shrink-0 font-mono text-[10px] text-faint">
          {keys}
        </span>
      )}
      {more && (
        <span aria-hidden="true" className="shrink-0 text-faint">
          ›
        </span>
      )}
    </span>
  );
}

function ItemMenuLine({ action, onRun }: { action: ResolvedAction; onRun: () => void }): JSX.Element {
  const why = useId();
  return (
    <>
      {/* Its name is its label: the icon and keys are decoration, and the
          reason sits under it (its description, and its tooltip). */}
      <MenuItem
        testId={`action-${action.id}`}
        onClick={onRun}
        disabled={!!action.reason}
        title={action.reason}
        describedBy={action.reason ? why : undefined}
        danger={action.destructive}
      >
        <Line icon={action.icon} label={action.label} keys={action.shortcut} />
      </MenuItem>
      {action.reason && (
        <span id={why} aria-hidden="true" className="-mt-0.5 block pb-1 pl-[30px] pr-2 text-[10px] leading-snug text-faint">
          {action.reason}
        </span>
      )}
    </>
  );
}

/** The menu's lines: groups split by a line, a submenu's actions in it. */
export function ItemMenuLines({ actions, run }: { actions: ResolvedAction[]; run: (action: ResolvedAction) => () => void }): JSX.Element {
  const entries = menuEntries(actions);
  return (
    <>
      {entries.map((entry, index) => (
        <Fragment key={entry.kind === "action" ? entry.action.id : `submenu-${entry.id}`}>
          {index > 0 && entries[index - 1]!.group !== entry.group && <MenuSeparator />}
          {entry.kind === "action" ? (
            <ItemMenuLine action={entry.action} onRun={run(entry.action)} />
          ) : (
            <MenuSubmenu label={entry.label} testId={`submenu-${entry.id}`} line={<Line icon={entry.icon} label={entry.label} more />}>
              {entry.actions.map((action) => (
                <ItemMenuLine key={action.id} action={action} onRun={run(action)} />
              ))}
            </MenuSubmenu>
          )}
        </Fragment>
      ))}
    </>
  );
}
