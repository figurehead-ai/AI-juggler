/**
 * The pool inspector's sections: the shared Section (components/Section,
 * D-126) in the inspector's divided look, its headings one level under
 * the inspector's title; and the shared FoldSection. The pool's
 * inspector, its setup sections and Settings › Agent access use these.
 */
import type { ComponentProps, JSX } from "react";

import { Section as SharedSection } from "../../components/Section";

export { FoldSection } from "../../components/Section";

export function Section(props: Omit<ComponentProps<typeof SharedSection>, "divided" | "as">): JSX.Element {
  return <SharedSection {...props} divided as="h4" />;
}
