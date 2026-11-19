/**
 * D-126 — the kit's one field look (DateField, TimeField): filled, a
 * hairline edge, 10 px corners; while it's typed in, the accent's edge
 * and a soft halo; a field whose words can't be read, a danger edge.
 */
export const FIELD_SHELL = "inline-flex items-center rounded-[10px] bg-fill text-ink inset-ring-1 focus-within:inset-ring-[1.5px] focus-within:ring-4 focus-within:ring-accent-soft";

/** The field's edge: hairline, or danger when its words can't be read. */
export function fieldEdge(error: boolean): string {
  return error ? "inset-ring-danger/70 focus-within:inset-ring-danger" : "inset-ring-hairline/75 focus-within:inset-ring-accent";
}
