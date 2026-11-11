/**
 * AJ-E3104: a CLI App's launch settings, read once for its Launch settings
 * tab and the tab's count, and read again after the tab adds or deletes one.
 */
import { useCallback, useEffect, useState } from "react";

import { errorText } from "../../lib/bridge";
import { launchRecipesList } from "../../lib/ipc/launch";
import { showToast } from "../../components/Toast";
import type { LaunchRecipeInfo } from "../../types/generated/LaunchRecipeInfo";

export function useLaunchSettings(folderId: string | null): { recipes: LaunchRecipeInfo[] | null; reload: () => void } {
  const [read, setRead] = useState<{ folderId: string; recipes: LaunchRecipeInfo[] } | null>(null);
  const reload = useCallback(() => {
    if (folderId === null) return;
    launchRecipesList().then(
      (rows) => setRead({ folderId, recipes: rows.filter((row) => row.account_id === folderId) }),
      (cause: unknown) => (setRead({ folderId, recipes: [] }), showToast({ id: "launch-settings", tone: "error", message: `Couldn't read its launch settings: ${errorText(cause)}` })),
    );
  }, [folderId]);
  useEffect(reload, [reload]);
  return { recipes: read !== null && read.folderId === folderId ? read.recipes : null, reload };
}
