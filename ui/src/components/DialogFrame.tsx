/**
 * 0.5.0 (D-142, DESIGN-SYSTEM §4.2, GAPS A1–A2) — the one frame every
 * dialog, sheet and wizard uses, so its way out can never be scrolled
 * away. The maintainer, on the Move or Copy dialog: "The buttons should
 * always be sticky and can't be scrolled out of the way."
 *
 * - `DialogOverlay`: the dimmed window, drawn in the page's body (a sheet
 *   opened inside a panel that makes its own layer was clipped by it), a
 *   16 px gutter, never centring what is taller than the window.
 * - `DialogFrame`: the card, capped at min(92vh, 760px) and the window
 *   less its gutter, in a size class (S 440, M 560 or 680 with cards,
 *   L 768, XL 1024). Its parts, each marked `data-dialog-part`:
 *   1. the header, fixed: title, close, step marks;
 *   2. the body, the only part that scrolls (vertically): a hairline and
 *      a soft shadow at its top once scrolled, its foot fading while more
 *      is below (`.scroll-edge`, `data-scrolled`, `data-more`);
 *   3. the footer, fixed: a top border that shows while the body scrolls,
 *      the reason or note line, the buttons (Cancel … the one primary).
 * - `DialogActions`: a row of actions where it must stay inside the body
 *   (a multi-step flow's own step parts); it sticks to the body's foot.
 * - `DialogHeading`: a title where it must stay inside the body (a flow
 *   whose phases each draw their own); it sticks to the body's top.
 *
 * Measured by tests/visual/dialog-fit.spec.ts (VALIDATION N1) at 1024×680
 * and 1280×800, both themes.
 */
import { useLayoutEffect, useRef, useState, type HTMLAttributes, type JSX, type ReactNode, type Ref, type RefObject } from "react";
import { createPortal } from "react-dom";

/** The overlay's default: the app's dim, a 16 px gutter, the card centred
 * while it fits (the frame's cap keeps it inside). */
export const OVERLAY = "fixed inset-0 z-50 flex items-center justify-center bg-canvas/70 p-4 backdrop-blur-sm";

/** The size classes (DESIGN-SYSTEM §4.2). */
export type DialogSize = "S" | "M" | "M-cards" | "L" | "XL";
const WIDTH: Record<DialogSize, string> = { S: "max-w-[440px]", M: "max-w-[560px]", "M-cards": "max-w-[680px]", L: "max-w-[768px]", XL: "max-w-[1024px]" };

/** The dimmed window under a dialog, drawn in the body. */
export function DialogOverlay({ className = OVERLAY, children, ref, ...rest }: { className?: string; children: ReactNode; ref?: Ref<HTMLDivElement> } & Omit<HTMLAttributes<HTMLDivElement>, "className" | "children">): JSX.Element {
  const overlay = (
    <div {...rest} ref={ref} className={className} data-dialog-overlay>
      {children}
    </div>
  );
  return typeof document === "undefined" ? overlay : createPortal(overlay, document.body);
}

/** Whether a box has more above (scrolled) and below (not at its end). */
export function useScrollEdges(ref: RefObject<HTMLElement | null>): { above: boolean; below: boolean } {
  const [edges, setEdges] = useState({ above: false, below: false });
  useLayoutEffect(() => {
    const node = ref.current;
    if (!node) return undefined;
    const read = () => {
      const above = node.scrollTop > 1;
      const below = node.scrollHeight - node.clientHeight - node.scrollTop > 1;
      setEdges((was) => (was.above === above && was.below === below ? was : { above, below }));
    };
    read();
    node.addEventListener("scroll", read, { passive: true });
    const sizes = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(read);
    const watch = () => {
      sizes?.observe(node);
      for (const child of node.children) sizes?.observe(child);
    };
    watch();
    const children = typeof MutationObserver === "undefined" ? null : new MutationObserver(() => (watch(), read()));
    children?.observe(node, { childList: true });
    return () => {
      node.removeEventListener("scroll", read);
      sizes?.disconnect();
      children?.disconnect();
    };
  }, [ref]);
  return edges;
}

export function DialogFrame({
  frameRef,
  size,
  sheet = false,
  className = "",
  header,
  headerClassName = "",
  footer,
  footerClassName = "flex flex-wrap items-center justify-end gap-2 px-5 py-3",
  bodyClassName = "",
  bodyTestId,
  bodyProps,
  headerTestId,
  footerTestId,
  children,
  ...rest
}: {
  frameRef?: Ref<HTMLDivElement>;
  /** Its width class; none where `className` sets its own width. */
  size?: DialogSize;
  /** A window sheet (a reading view, a long flow): the window's height
   * less its gutter, not the dialog cap. */
  sheet?: boolean;
  /** The card's look (its column, cap and clipping are the frame's). */
  className?: string;
  /** Stays at the top: the title, the close, step marks. */
  header?: ReactNode;
  headerClassName?: string;
  /** Stays at the foot: the reason line and the buttons. */
  footer?: ReactNode;
  footerClassName?: string;
  /** The body's padding and layout. */
  bodyClassName?: string;
  bodyTestId?: string;
  /** More on the body (a drop zone's handlers). */
  bodyProps?: Omit<HTMLAttributes<HTMLDivElement>, "className" | "children">;
  headerTestId?: string;
  footerTestId?: string;
  children: ReactNode;
} & Omit<HTMLAttributes<HTMLDivElement>, "className" | "children">): JSX.Element {
  const bodyRef = useRef<HTMLDivElement | null>(null);
  const { above, below } = useScrollEdges(bodyRef);
  const scrolls = above || below;
  return (
    <div
      {...rest}
      ref={frameRef}
      // A caller's own cap (a dialog held near the top) replaces the default.
      className={`flex ${/\bmax-h-/.test(className) ? "" : sheet ? "max-h-full" : "max-h-[min(92vh,760px,calc(100vh_-_2rem))]"} min-h-0 w-full flex-col overflow-hidden ${size ? WIDTH[size] : ""} ${className}`}
      data-dialog-frame
    >
      <div className={`shrink-0 ${headerClassName}`} data-dialog-part="header" data-testid={headerTestId}>
        {header}
      </div>
      <div
        {...bodyProps}
        ref={bodyRef}
        className={`scroll-edge group/dialog min-h-0 flex-1 overflow-x-hidden overflow-y-auto overscroll-contain ${bodyClassName}`}
        data-dialog-part="body"
        data-scrolled={above || undefined}
        data-more={below || undefined}
        data-testid={bodyTestId}
      >
        {children}
      </div>
      {footer && (
        <div
          className={`shrink-0 border-t transition-colors ${scrolls ? "border-hairline" : "border-transparent"} ${footerClassName}`}
          data-dialog-part="footer"
          data-testid={footerTestId}
        >
          {footer}
        </div>
      )}
    </div>
  );
}

/**
 * Actions where they must stay inside the body (a flow's step part that
 * owns them): they stick to the body's foot, over the surface, with a
 * hairline above while more is below.
 */
export function DialogActions({ children, className = "justify-end", testId }: { children: ReactNode; className?: string; testId?: string }): JSX.Element {
  return (
    <div
      className={
        "sticky bottom-0 z-10 flex flex-wrap items-center gap-2 border-t border-transparent bg-surface py-2 group-data-[more]/dialog:border-hairline " + className
      }
      data-dialog-actions
      data-testid={testId}
    >
      {children}
    </div>
  );
}

/** A title where it must stay inside the body: it sticks to the body's
 * top (over the body's own top padding), with a hairline under it once the
 * body is scrolled. */
export function DialogHeading({ id, className = "type-section", children }: { id?: string; className?: string; children: ReactNode }): JSX.Element {
  return (
    <h3 id={id} className={`sticky -top-5 z-10 -mt-5 -mb-2 bg-surface pt-5 pb-2 group-data-[scrolled]/dialog:shadow-[0_1px_0_var(--color-hairline)] ${className}`} data-dialog-heading>
      {children}
    </h3>
  );
}
