/**
 * Whether the person would rather have less motion (the system's Reduce
 * motion), kept current as they change it: the pool canvas's edge
 * pulses (a brief glow instead of a moving dot).
 */
import { useEffect, useState } from "react";

export function useReducedMotion(): boolean {
  const query = typeof window !== "undefined" && window.matchMedia ? window.matchMedia("(prefers-reduced-motion: reduce)") : null;
  const [reduced, setReduced] = useState(query?.matches ?? false);
  useEffect(() => {
    const change = () => setReduced(query?.matches ?? false);
    query?.addEventListener?.("change", change);
    return () => query?.removeEventListener?.("change", change);
  }, [query]);
  return reduced;
}
