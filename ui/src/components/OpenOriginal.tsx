/**
 * rc21: the way out beside a greyed Resume or Continue: the conversation
 * a copy was made from, as the switch that made it recorded it. (When AI
 * Juggler doesn't know it, the greyed control's reason says how to find
 * it.)
 */
import type { JSX } from "react";

import { Button } from "./primitives";
import type { ResumeRefusalInfo } from "../lib/resume-refusal";

export function OpenOriginal({
  refusal,
  onOpen,
  size = "sm",
}: {
  refusal: ResumeRefusalInfo | null | undefined;
  /** Opens a conversation by its account and id. */
  onOpen?: (accountId: string, id: string, projectId: string | null) => void;
  size?: "sm" | "md";
}): JSX.Element | null {
  const original = refusal?.original;
  if (!original || !onOpen) return null;
  const where = original.account_label ? ` on ${original.account_label}` : "";
  return (
    <Button
      size={size}
      variant="secondary"
      aria-description={`The conversation it was copied from${where}`}
      onClick={() => onOpen(original.account_id, original.id, null)}
      data-testid="open-original"
    >
      Open the original
    </Button>
  );
}
