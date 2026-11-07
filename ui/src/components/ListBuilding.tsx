/**
 * A first start builds the conversation list from every transcript on the
 * Mac (about 40 s on a long history; later starts read only what changed).
 * Meanwhile Home and the rail say so in one quiet line, where the lists
 * would be, and it goes away by itself once the list is built.
 */
import type { JSX } from "react";
import { useConversations } from "../lib/conversations";

export function ListBuilding({ place }: { place: "home" | "rail" }): JSX.Element | null {
  const { building } = useConversations();
  if (!building) return null;
  return (
    <output data-testid={`list-building-${place}`} className={place === "rail" ? "block px-3 py-1 text-label text-muted" : "block text-meta text-ink-3"}>
      Building the conversation list…
    </output>
  );
}
