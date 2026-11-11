/**
 * AJ-E3205 R3: where "Measure its limits" leads. When measuring can be
 * switched on for the App's folder (an account used where it is), its
 * App's Settings, where the switch is; otherwise its App's Overview, where
 * New session here starts the session that reports them.
 */
import { captureStatus } from "./ipc/capture";
import { rememberTab } from "../pages/registry";

/** Opens the App of `folderId` on the tab where its limits get measured. */
export async function openWhereMeasured(folderId: string, open: () => void): Promise<void> {
  const offered = await captureStatus(folderId).then(
    (status) => status.offered,
    () => false,
  );
  rememberTab("app", offered ? "settings" : "overview");
  open();
}
