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
import { useEffect, type JSX } from "react";

import { shellOnlyWhy, useAgentToolsWhy } from "../lib/agent-tools";
import { useAgentBridge } from "../lib/ipc/agent-bridge";
import { useTools } from "../lib/tools";
import { keyboardFocus, useWhyTip } from "./WhyTip";

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
  return shellOnlyWhy(spec, appWhy, bridge, account, project);
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

export function ShellOnlyChip({ why, inRow }: { why: string; inRow: boolean }): JSX.Element {
  const { ref, show, hide, shown, tipId, tip } = useWhyTip<HTMLSpanElement>(why, "shell-only-tip");
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
  }, [inRow, ref, show, hide]);
  return (
    <span
      ref={ref}
      tabIndex={inRow ? undefined : 0}
      aria-description={why}
      aria-describedby={shown ? tipId : undefined}
      onPointerEnter={show}
      onPointerLeave={hide}
      onFocus={inRow ? undefined : (event) => keyboardFocus(event.currentTarget) && show()}
      onBlur={inRow ? undefined : hide}
      className="shrink-0 rounded-md bg-surface-2 px-1.5 text-label text-ink-2 outline-none focus-visible:ring-1 focus-visible:ring-accent/60"
      data-testid="shell-only"
    >
      Shell only
      {tip}
    </span>
  );
}
