/**
 * AJ-E1504 R2 (0.5.1 F89): whether the page is folded to its header line
 * (⌘⇧I). PageShell provides it; a body with a side column (a session's
 * SessionColumns) hides that column, so the conversation gets the room.
 */
import { createContext } from "react";

export const PageFolded = createContext(false);
