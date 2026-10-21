/**
 * 0.0.11 walk (Q3): which tools don't report their usage limits, from the
 * backend's capabilities (each adapter's quota feed) — never a check on
 * the tool's id here. Until the answer comes, or if the call fails,
 * every tool keeps its limit rows.
 */
import { useEffect, useState } from "react";
import { providersCapabilities } from "./bridge";
import { limitProviders, type InPlaceSupport } from "./providers";

export function useToolsWithoutLimits(): InPlaceSupport {
  const [noLimits, setNoLimits] = useState<InPlaceSupport>("unknown");
  useEffect(() => {
    let alive = true;
    void providersCapabilities().then(
      (rows) => alive && setNoLimits(limitProviders(rows)),
      () => undefined,
    );
    return () => {
      alive = false;
    };
  }, []);
  return noLimits;
}
