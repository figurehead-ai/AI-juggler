/**
 * The header's More menu: the screens that don't need a tab of their own,
 * and (AJ-E1533) the user's tools, each opening its page. Closes on a
 * click elsewhere or Escape, like any menu.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { ToolMark } from "./ToolMark";
import { useTools } from "../lib/tools";

export type MoreItem<Id extends string> = { id: Id; label: string; hint: string };

export function MoreMenu<Id extends string>({
  items,
  current,
  onPick,
  toolIds,
  onOpenTool,
}: {
  items: MoreItem<Id>[];
  /** The screen shown now, when it's one of these. */
  current: Id | null;
  onPick: (id: Id) => void;
  /** The tools the user has accounts on. */
  toolIds: string[];
  onOpenTool: (id: string) => void;
}): JSX.Element {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement | null>(null);
  const tools = useTools().filter((tool) => toolIds.includes(tool.id));
  useEffect(() => {
    if (!open) return;
    const onDown = (event: MouseEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) setOpen(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);
  const pick = (run: () => void) => () => {
    setOpen(false);
    run();
  };
  return (
    <div className="relative" ref={ref}>
      <button
        type="button"
        aria-expanded={open}
        aria-controls="nav-more"
        onClick={() => setOpen((was) => !was)}
        className={
          "rounded-md px-2 py-1.5 text-xs font-medium transition-colors xl:px-2.5 " +
          (items.some((item) => item.id === current) ? "bg-surface text-ink sheen" : "text-muted hover:bg-surface/60 hover:text-ink")
        }
      >
        More
      </button>
      {open && (
        <div id="nav-more" className="absolute right-0 top-full z-40 mt-1.5 flex w-72 flex-col rounded-xl border border-hairline bg-surface p-1.5 shadow-xl">
          {items.map((item) => (
            <button
              key={item.id}
              type="button"
              aria-current={current === item.id ? "page" : undefined}
              onClick={pick(() => onPick(item.id))}
              className="flex flex-col items-start gap-0.5 rounded-lg px-3 py-2 text-left hover:bg-surface-2"
            >
              <span className="text-xs font-medium text-ink">{item.label}</span>
              <span className="text-[11px] text-muted">{item.hint}</span>
            </button>
          ))}
          {tools.length > 0 && (
            <>
              <p className="mt-1 border-t border-hairline px-3 pb-1 pt-2 text-[10px] font-medium uppercase tracking-wider text-muted">Tools</p>
              {tools.map((tool) => (
                <button
                  key={tool.id}
                  type="button"
                  data-testid="more-tool"
                  onClick={pick(() => onOpenTool(tool.id))}
                  className="flex items-center gap-2 rounded-lg px-3 py-1.5 text-left text-xs text-ink hover:bg-surface-2"
                >
                  <ToolMark tool={tool.id} size={14} className="text-muted" />
                  {tool.display_name}
                </button>
              ))}
            </>
          )}
        </div>
      )}
    </div>
  );
}
