/**
 * D-62 (the P8 walk): a pool member whose tool has no agent tools yet
 * works through its shell only. It can't read the pool, post or take a
 * task, so it says so wherever a member is shown: its card on the canvas,
 * its row in the List view, its inspector. So does one whose App can't
 * take them (AJ-E3002 R3), with its adapter's reason, and one in a
 * project or App where the user switched them off (Settings › Agent
 * access › "Where they're off"; the rc18 walk, E3002-R3 step 12).
 *
 * The rc19 walk: the chip never said why. Its reason was only a native
 * `title`, which the app's web view shows after a still hover in a key
 * window, never on keyboard focus, and which a draggable canvas node
 * rarely gets. So the chip carries its reason as its description and in
 * a tip of its own, shown on hover and on keyboard focus (Escape closes
 * it), drawn over the page so a clipped row or a zoomed canvas can't cut
 * it; where there's room the reason is written out too.
 */
import { useCallback, useEffect, useId, useRef, useState, type JSX } from "react";
import { createPortal } from "react-dom";

import { switchedOffWhy, useAgentToolsWhy } from "../lib/agent-tools";
import { useAgentBridge } from "../lib/ipc/agent-bridge";
import { useTools } from "../lib/tools";

const SHELL_ONLY_WHY = "Its tool has no agent tools yet: it works through its shell, and can't read the pool, post or take a task.";

/** The tip's width (`w-64`), kept inside the window. */
const TIP_W = 256;

/** Why a member works through its shell only, or `null` when it gets
 * the agent tools: its tool has none (a tool not described yet: nothing
 * claimed either way), its App's launches can't take them (its
 * adapter's words, AJ-E3002 R3), or the user switched them off, for
 * every session or its project or App (as the launch decides; nothing
 * claimed while that loads or before the question is answered). */
export function useShellOnly(tool: string, account?: string | null, project?: string | null): string | null {
  const spec = useTools().find((one) => one.id === tool);
  const appWhy = useAgentToolsWhy(account);
  const bridge = useAgentBridge();
  if (!spec) return null;
  if (!spec.supports.agent_tools) return SHELL_ONLY_WHY;
  return appWhy ?? switchedOffWhy(bridge, account, project);
}

/** Whether focus came from the keyboard (a click's focus shows no tip). */
function keyboardFocus(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  try {
    return target.matches(":focus-visible");
  } catch {
    return true;
  }
}

/**
 * The "Shell only" chip. `inRow`: it sits in a row's button or on a
 * canvas node, which keeps the one tab stop; its tip shows while that
 * row or node has the keyboard's focus. Elsewhere the chip is a tab stop
 * of its own.
 */
export function ShellOnly({ tool, account, project, inRow = false }: { tool: string; account?: string | null; project?: string | null; inRow?: boolean }): JSX.Element | null {
  const why = useShellOnly(tool, account, project);
  if (!why) return null;
  return <ShellOnlyChip why={why} inRow={inRow} />;
}

function ShellOnlyChip({ why, inRow }: { why: string; inRow: boolean }): JSX.Element {
  const ref = useRef<HTMLSpanElement | null>(null);
  const tipId = `shell-only-${useId().replace(/:/g, "")}`;
  const [at, setAt] = useState<{ left: number; top: number } | null>(null);
  const show = useCallback(() => {
    const box = ref.current?.getBoundingClientRect();
    if (!box) return;
    setAt({ left: Math.max(8, Math.min(box.left, window.innerWidth - TIP_W - 8)), top: box.bottom + 6 });
  }, []);
  const hide = useCallback(() => setAt(null), []);
  useEffect(() => {
    if (!inRow) return undefined;
    const host = ref.current?.closest<HTMLElement>("[data-node], button");
    if (!host) return undefined;
    const focusIn = (event: FocusEvent) => {
      if (keyboardFocus(event.target)) show();
    };
    host.addEventListener("focusin", focusIn);
    host.addEventListener("focusout", hide);
    return () => {
      host.removeEventListener("focusin", focusIn);
      host.removeEventListener("focusout", hide);
    };
  }, [inRow, show, hide]);
  useEffect(() => {
    if (!at) return undefined;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") hide();
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("scroll", hide, true);
    window.addEventListener("wheel", hide, { passive: true });
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("scroll", hide, true);
      window.removeEventListener("wheel", hide);
    };
  }, [at, hide]);
  return (
    <span
      ref={ref}
      tabIndex={inRow ? undefined : 0}
      aria-description={why}
      aria-describedby={at ? tipId : undefined}
      onPointerEnter={show}
      onPointerLeave={hide}
      onFocus={inRow ? undefined : (event) => keyboardFocus(event.currentTarget) && show()}
      onBlur={inRow ? undefined : hide}
      className="shrink-0 rounded-md bg-surface-2 px-1.5 text-label text-ink-2 outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
      data-testid="shell-only"
    >
      Shell only
      {at &&
        createPortal(
          <span
            id={tipId}
            role="tooltip"
            style={{ left: at.left, top: at.top, width: TIP_W }}
            className="pointer-events-none fixed z-50 rounded-lg border border-hairline bg-surface px-2.5 py-1.5 text-label leading-snug text-ink shadow-card"
            data-testid="shell-only-tip"
          >
            {why}
          </span>,
          document.body,
        )}
    </span>
  );
}
