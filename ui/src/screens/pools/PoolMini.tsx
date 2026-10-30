/**
 * A pool's canvas in small (the Pools screen's cards): its open members as
 * chips where they sit on its canvas (where you put them, else the
 * automatic layout), the lead in the accent colour, one that waits on you
 * in amber. Drawn, not interactive: the card around it opens the pool.
 */
import type { JSX } from "react";

import { bounds, layout, NODE_H, NODE_W, type Point } from "../../pages/pool/layout";

export type MiniNode = { id: string; role: string; alert: boolean; working: boolean };

export function PoolMini({ nodes, placed, height = 92 }: { nodes: MiniNode[]; placed: ReadonlyMap<string, Point>; height?: number }): JSX.Element {
  const points = layout(
    nodes.map((node) => ({ id: node.id, role: node.role })),
    placed,
  );
  const box = bounds(points.values());
  const pad = 60;
  const view = box ? `${box.x - pad} ${box.y - pad} ${Math.max(box.w + pad * 2, 900)} ${Math.max(box.h + pad * 2, 420)}` : "0 0 900 420";
  // Wide enough that one node never fills the card: centre what's there.
  const [vx, vy, vw, vh] = view.split(" ").map(Number) as [number, number, number, number];
  const shiftX = box ? (vw - (box.w + pad * 2)) / 2 : 0;
  const shiftY = box ? (vh - (box.h + pad * 2)) / 2 : 0;
  return (
    <svg
      viewBox={`${vx - shiftX} ${vy - shiftY} ${vw} ${vh}`}
      preserveAspectRatio="xMidYMid meet"
      className="block w-full rounded-lg border border-hairline/70 bg-canvas"
      style={{ height, backgroundImage: "radial-gradient(var(--color-hairline) 1px, transparent 1px)", backgroundSize: "12px 12px" }}
      aria-hidden="true"
      data-testid="pool-mini"
    >
      {nodes.length === 0 && (
        <rect x={vx - shiftX + vw / 2 - NODE_W / 2} y={vy - shiftY + vh / 2 - NODE_H / 2} width={NODE_W} height={NODE_H} rx="22" fill="none" strokeWidth="6" strokeDasharray="18 14" className="stroke-hairline" />
      )}
      {nodes.map((node) => {
        const at = points.get(node.id);
        if (!at) return null;
        const tone = node.alert ? "stroke-warning" : node.role === "lead" ? "stroke-accent" : "stroke-hairline";
        return (
          <g key={node.id} data-mini-node={node.role}>
            <rect x={at.x} y={at.y} width={NODE_W} height={NODE_H} rx="22" strokeWidth={node.alert || node.role === "lead" ? 8 : 5} className={"fill-surface-2 " + tone} />
            <rect x={at.x + 28} y={at.y + 34} width={NODE_W * 0.55} height="16" rx="8" className="fill-surface-3" />
            <circle cx={at.x + 36} cy={at.y + 78} r="9" className={node.alert ? "fill-warning" : node.working ? "fill-live" : "fill-faint"} />
          </g>
        );
      })}
    </svg>
  );
}
