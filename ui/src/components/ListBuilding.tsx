/**
 * A first start builds the conversation list from every transcript on the
 * Mac (later starts read only what changed). Meanwhile Home and the rail
 * say so in one quiet line, where the lists would be: how far it is once
 * the files are counted, and, when a pass failed, that it did, with a Try
 * again. It goes away by itself once the list is built.
 */
import type { JSX } from "react";
import { useConversations } from "../lib/conversations";

export function ListBuilding({ place }: { place: "home" | "rail" }): JSX.Element | null {
  const { build, buildAgain } = useConversations();
  if (build === null) return null;
  const testId = `list-building-${place}`;
  const tone = place === "rail" ? "block px-3 py-1 text-label text-muted" : "block text-meta text-ink-3";
  if (build.failed) {
    return (
      <output data-testid={testId} className={place === "rail" ? "block px-3 py-1 text-label text-ink-2" : "block text-meta text-ink-2"}>
        Couldn't read the conversation list. It tries again by itself.{" "}
        <button
          type="button"
          data-testid={`${testId}-again`}
          onClick={buildAgain}
          className="rounded text-accent-hi underline-offset-2 outline-none hover:underline focus-visible:ring-1 focus-visible:ring-accent/60"
        >
          Try again
        </button>
      </output>
    );
  }
  return (
    <output data-testid={testId} className={tone}>
      {build.total > 0 ? `Reading conversations: ${build.read.toLocaleString("en-US")} of ${build.total.toLocaleString("en-US")}` : "Building the conversation list…"}
    </output>
  );
}
