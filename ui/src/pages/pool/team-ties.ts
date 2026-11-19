/**
 * A team that hasn't written yet keeps its shape on the canvas (the rc3a
 * walk: the setup canvas drew the lead's lines, and they went the moment
 * sessions joined, leaving the cards afloat; the mock's running view has
 * them): a quiet tie from its lead to each member with no line either way.
 *
 * Only a tie that reaches its member clear of every other card (one run
 * behind a row of cards to the row under it read as clutter, not shape).
 * A member under the lead gets the setup canvas's own curve, from the
 * lead's foot to the member's head; one beside or above it, the edge's.
 */
import type { CanvasEdgeInfo } from "../../lib/ipc/pool-canvas";
import { crosses, edgeCurve, NODE_H, NODE_W, type LabelRoom, type Point } from "./layout";
import { edgeKey } from "./useCanvas";

export type Tie = { key: string; d: string };

export function teamTies(edges: readonly CanvasEdgeInfo[], points: ReadonlyMap<string, Point>, lead: string | null, room: LabelRoom): Tie[] {
  const from = lead ? points.get(lead) : undefined;
  if (!lead || !from) return [];
  const linked = new Set(edges.flatMap((edge) => [edgeKey(edge.from_session_id, edge.to_session_id), edgeKey(edge.to_session_id, edge.from_session_id)]));
  return [...points]
    .filter(([id]) => id !== lead && !linked.has(edgeKey(lead, id)))
    .flatMap(([id, to]) => {
      const around = [...points].filter(([other]) => other !== lead && other !== id).map(([, at]) => at);
      const curve = edgeCurve(from, to, false, around, [], room);
      if (crosses(curve.start, curve.control, curve.end, around)) return [];
      const under = to.y >= from.y + NODE_H;
      const [x1, y1, x2, y2] = [from.x + NODE_W / 2, from.y + NODE_H, to.x + NODE_W / 2, to.y];
      return [{ key: edgeKey(lead, id), d: under ? `M${x1} ${y1} C${x1} ${y1 + 60} ${x2} ${y2 - 60} ${x2} ${y2}` : curve.d }];
    });
}
