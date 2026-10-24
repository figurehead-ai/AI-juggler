/**
 * A settings card's title. On its own (a sheet, a test) it shows; inside
 * a Settings section (AJ-E2110), whose header already names it, it stays
 * for screen readers and in-page links only, so the name isn't said
 * twice on screen.
 */
import { createContext, forwardRef, useContext, type HTMLAttributes } from "react";

/** True inside a Settings pane's section (its header names the card). */
export const InSettingsSection = createContext(false);

export const CardTitle = forwardRef<HTMLHeadingElement, HTMLAttributes<HTMLHeadingElement>>(function CardTitle(
  { className = "", ...props },
  ref,
) {
  const inSection = useContext(InSettingsSection);
  return <h3 ref={ref} className={inSection ? "sr-only" : `text-sm font-semibold ${className}`} {...props} />;
});
