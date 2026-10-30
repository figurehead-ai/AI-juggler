/**
 * The pool canvas's view tools, bottom left: zoom out, the zoom (a click
 * goes back to 100 %, where the nodes are their full cards), zoom in,
 * fit, and Tidy up (the automatic layout again).
 */
import type { JSX } from "react";

import { IconFit, IconTidy, IconZoomIn, IconZoomOut } from "../../components/icons-canvas";

const STEP = 1.2;

export function CanvasToolbar({ scale, onZoom, onFit, onTidy }: { scale: number; onZoom: (factor: number) => void; onFit: () => void; onTidy: () => void }): JSX.Element {
  const percent = Math.round(scale * 100);
  return (
    <div data-canvas-toolbar className="absolute bottom-3 left-3 flex items-center gap-1 rounded-xl border border-hairline bg-surface p-1 shadow-card" role="toolbar" aria-label="Canvas view">
      <ToolButton label="Zoom out" onClick={() => onZoom(1 / STEP)}>
        <IconZoomOut width={16} height={16} />
      </ToolButton>
      <button
        type="button"
        onClick={() => onZoom(1 / scale)}
        className="h-7 w-12 rounded-md font-mono text-label text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
        aria-label={`${percent}%, back to 100%`}
        title={percent === 100 ? "100 %: each member as its full card" : "Back to 100 %: each member as its full card"}
        data-testid="canvas-zoom"
      >
        {percent}%
      </button>
      <ToolButton label="Zoom in" onClick={() => onZoom(STEP)}>
        <IconZoomIn width={16} height={16} />
      </ToolButton>
      <ToolButton label="Fit to screen" onClick={onFit} testId="canvas-fit">
        <IconFit width={16} height={16} />
      </ToolButton>
      <span className="mx-0.5 h-5 w-px bg-hairline" aria-hidden="true" />
      <button
        type="button"
        onClick={onTidy}
        className="flex h-7 items-center gap-1.5 rounded-md px-2 text-label text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60"
        data-testid="canvas-tidy"
        title="Lay the pool out again: the lead on top, the rest under it"
      >
        <IconTidy width={16} height={16} aria-hidden="true" />
        Tidy up
      </button>
    </div>
  );
}

function ToolButton({ label, onClick, testId, children }: { label: string; onClick: () => void; testId?: string; children: JSX.Element }): JSX.Element {
  return (
    <button type="button" onClick={onClick} aria-label={label} title={label} data-testid={testId} className="flex size-7 items-center justify-center rounded-md text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-1 focus-visible:ring-accent/60">
      {children}
    </button>
  );
}
