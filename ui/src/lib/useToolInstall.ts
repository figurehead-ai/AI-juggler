/**
 * AJ-E408: what this Mac has of a tool (installed version, verified
 * range), for its page.
 */
import { useEffect, useState } from "react";

/** What's installed for one tool: null while checking (nothing shown),
 * "failed" when the check failed or doesn't know the tool. The result is
 * kept per tool, so another tool's version never shows while checking. */
export function useToolInstall(
  providerId: string | null,
): import("./bridge").ProviderInstallData | "failed" | null {
  const [result, setResult] = useState<{
    providerId: string;
    install: import("./bridge").ProviderInstallData | "failed";
  } | null>(null);
  useEffect(() => {
    if (!providerId) return;
    let alive = true;
    void import("./bridge")
      .then((bridge) => bridge.providersDiscover())
      .then(
        (rows) => {
          if (alive) setResult({ providerId, install: rows.find((row) => row.provider === providerId) ?? "failed" });
        },
        () => {
          if (alive) setResult({ providerId, install: "failed" });
        },
      );
    return () => {
      alive = false;
    };
  }, [providerId]);
  return result && result.providerId === providerId ? result.install : null;
}
