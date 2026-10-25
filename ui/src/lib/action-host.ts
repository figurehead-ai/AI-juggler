/**
 * AJ-E1537 — what the screens can do for the action registry
 * (`lib/actions.ts`), by item type. Each piece is optional: the screen
 * that owns it provides it, and an action whose piece isn't there
 * doesn't show.
 */
import type { AccountInfo, InstanceInfo, ProjectInfo } from "./bridge";
import type { ToolSpecInfo } from "../types/generated/ToolSpecInfo";
import type { ConversationInfo } from "../types/generated/ConversationInfo";

/** What the screens can do, by item type. Each piece is optional: the
 * screen that owns it provides it (an `ActionHostProvider`). */
export interface ActionHost {
  // Sessions
  openSession?: (row: InstanceInfo) => void;
  focusSession?: (row: InstanceInfo) => void | Promise<void>;
  continueSession?: (row: InstanceInfo) => void;
  renameSession?: (row: InstanceInfo) => void;
  pinSession?: (row: InstanceInfo) => void | Promise<void>;
  copyResumeCommand?: (row: InstanceInfo) => void | Promise<void>;
  revealSessionProject?: (row: InstanceInfo) => void;
  /** AJ-E1702: its pool (the pool sheet's "Move to pool"). */
  moveSessionToPool?: (row: InstanceInfo) => void;
  /** AJ-E1703: a pool's page, by its id. */
  openPool?: (poolId: string) => void;
  stopSession?: (row: InstanceInfo) => void;
  /** The rail's attention controls (snooze, mute). */
  snoozeSession?: (row: InstanceInfo, ms: number | null) => void;
  muteSession?: (row: InstanceInfo) => void;
  unsnoozeSession?: (row: InstanceInfo) => void;
  isSnoozed?: (row: InstanceInfo) => boolean;
  // Projects
  openProject?: (project: ProjectInfo) => void;
  newSessionIn?: (project: ProjectInfo) => void;
  openProjectIn?: (project: ProjectInfo, how: "editor" | "finder") => void | Promise<void>;
  /** The editor projects open in, when one is chosen (Settings). */
  editorName?: () => string | null;
  copyProjectPath?: (project: ProjectInfo) => void | Promise<void>;
  renameProject?: (project: ProjectInfo) => void;
  /** AJ-E1532: its colour, letters, emoji or icon. */
  markProject?: (project: ProjectInfo) => void;
  pinProject?: (project: ProjectInfo) => void | Promise<void>;
  relocateProject?: (project: ProjectInfo) => void | Promise<void>;
  forgetProject?: (project: ProjectInfo) => void;
  setProjectAccount?: (project: ProjectInfo) => void;
  /** AJ-E1507: the project's rules files and MCP servers, compared. */
  syncProject?: (project: ProjectInfo) => void;
  // Accounts
  openAccount?: (account: AccountInfo) => void;
  startSessionOn?: (account: AccountInfo) => void;
  /** Why a session can't start on it now (`null`: it can). */
  startBlockedWhy?: (account: AccountInfo) => string | null;
  signIn?: (account: AccountInfo) => void;
  editLaunchSettings?: (account: AccountInfo) => void;
  /** AJ-E2109: measure its limits (its page has the opt-in). */
  measureLimits?: (account: AccountInfo) => void;
  renameAccount?: (account: AccountInfo) => void;
  /** AJ-E1508: another account's usual settings, copied into this one. */
  copyAccountSettings?: (account: AccountInfo) => void;
  toggleDefaultAccount?: (account: AccountInfo) => void;
  removeAccount?: (account: AccountInfo) => void;
  // Tools
  openTool?: (tool: ToolSpecInfo) => void;
  copyUpdateCommand?: (tool: ToolSpecInfo) => void | Promise<void>;
  toolAccounts?: (tool: ToolSpecInfo) => void;
  // History
  openHistory?: (row: InstanceInfo) => void;
  replay?: (row: InstanceInfo) => void;
  continueHistory?: (row: InstanceInfo) => void;
  archive?: (row: InstanceInfo) => void | Promise<void>;
  unarchive?: (row: InstanceInfo) => void | Promise<void>;
  isArchived?: (row: InstanceInfo) => boolean;
  deleteHistory?: (row: InstanceInfo) => void;
  // This Mac
  openSessionById?: (id: string) => void;
  openProjectById?: (id: string) => void;
  // Conversations started outside AI Juggler (AJ-E1529)
  openConversation?: (conversation: ConversationInfo) => void;
  /** AJ-E2102: its app forward, by the app's own process. */
  goToConversation?: (conversation: ConversationInfo) => void | Promise<void>;
  continueConversation?: (conversation: ConversationInfo) => void | Promise<void>;
  copyConversationResume?: (conversation: ConversationInfo) => void | Promise<void>;
}
