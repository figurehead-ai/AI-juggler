/**
 * AJ-E1537, AJ-E2106 — the actions of what work runs on and in:
 * projects, accounts, tools, and This Mac's lines. The same three groups
 * as a session's menu: getting there, working on it, removing last.
 */
import type { ProjectInfo } from "./bridge";
import type { ActionSpec } from "./action-types";
import { has } from "./action-specs";

const FOLDER_GONE = (p: ProjectInfo) => (p.missing ? "Its folder isn't there any more" : null);

export const PROJECT: ActionSpec<"project">[] = [
  { id: "open", label: "Open its page", icon: "page", group: "go", rank: 10, applies: (_, h) => has(h.openProject), run: (p, h) => h.openProject!(p) },
  {
    id: "open-editor",
    label: (_, h) => (h.editorName?.() ? `Open in ${h.editorName()}` : "Open in Finder"),
    icon: (_, h) => (h.editorName?.() ? "editor" : "finder"),
    group: "go",
    rank: 20,
    applies: (_, h) => has(h.openProjectIn),
    blocked: FOLDER_GONE,
    run: (p, h) => h.openProjectIn!(p, "editor"),
  },
  {
    id: "reveal",
    label: "Show in Finder",
    icon: "finder",
    group: "go",
    rank: 30,
    applies: (_, h) => has(h.openProjectIn) && !!h.editorName?.(),
    blocked: FOLDER_GONE,
    run: (p, h) => h.openProjectIn!(p, "finder"),
  },
  {
    id: "new-session",
    label: "New session here",
    icon: "new",
    group: "work",
    rank: 10,
    applies: (_, h) => has(h.newSessionIn),
    blocked: FOLDER_GONE,
    run: (p, h) => h.newSessionIn!(p),
  },
  // Only a moved project can be relocated: left out otherwise.
  {
    id: "relocate",
    label: "Relocate…",
    icon: "relocate",
    group: "work",
    rank: 15,
    applies: (p, h) => has(h.relocateProject) && p.missing === true,
    run: (p, h) => h.relocateProject!(p),
  },
  { id: "copy-path", label: "Copy path", icon: "copy", group: "work", rank: 20, applies: (_, h) => has(h.copyProjectPath), run: (p, h) => h.copyProjectPath!(p) },
  { id: "rename", label: "Rename…", icon: "rename", group: "work", rank: 30, applies: (_, h) => has(h.renameProject), run: (p, h) => h.renameProject!(p) },
  {
    id: "pin",
    label: (p) => (p.pinned ? "Unpin" : "Pin to top"),
    icon: "pin",
    group: "work",
    rank: 40,
    applies: (_, h) => has(h.pinProject),
    run: (p, h) => h.pinProject!(p),
  },
  { id: "mark", label: "Change its mark…", icon: "mark", group: "work", rank: 50, applies: (_, h) => has(h.markProject), run: (p, h) => h.markProject!(p) },
  {
    id: "default-account",
    label: "Set its account…",
    icon: "account",
    group: "work",
    rank: 60,
    applies: (_, h) => has(h.setProjectAccount),
    run: (p, h) => h.setProjectAccount!(p),
  },
  {
    id: "sync",
    label: "Compare rules and servers…",
    icon: "compare",
    group: "work",
    rank: 70,
    applies: (_, h) => has(h.syncProject),
    blocked: FOLDER_GONE,
    run: (p, h) => h.syncProject!(p),
  },
  { id: "forget", label: "Forget…", icon: "remove", group: "stop", rank: 10, applies: (_, h) => has(h.forgetProject), run: (p, h) => h.forgetProject!(p) },
];

export const ACCOUNT: ActionSpec<"account">[] = [
  { id: "open", label: "Open its page", icon: "page", group: "go", rank: 10, applies: (_, h) => has(h.openAccount), run: (a, h) => h.openAccount!(a) },
  {
    id: "start",
    label: "Start a session",
    icon: "new",
    group: "work",
    rank: 10,
    applies: (_, h) => has(h.startSessionOn),
    blocked: (a, h) => h.startBlockedWhy?.(a) ?? null,
    run: (a, h) => h.startSessionOn!(a),
  },
  { id: "sign-in", label: "Sign in…", icon: "sign-in", group: "work", rank: 20, applies: (_, h) => has(h.signIn), run: (a, h) => h.signIn!(a) },
  {
    id: "launch-settings",
    label: "Edit launch settings",
    icon: "settings",
    group: "work",
    rank: 30,
    applies: (_, h) => has(h.editLaunchSettings),
    run: (a, h) => h.editLaunchSettings!(a),
  },
  // AJ-E2109: a profile whose sessions run elsewhere can report its limits.
  {
    id: "measure-limits",
    label: "Measure its limits…",
    icon: "info",
    group: "work",
    rank: 35,
    applies: (_, h) => has(h.measureLimits),
    run: (a, h) => h.measureLimits!(a),
  },
  { id: "rename", label: "Rename…", icon: "rename", group: "work", rank: 40, applies: (_, h) => has(h.renameAccount), run: (a, h) => h.renameAccount!(a) },
  {
    id: "copy-settings",
    label: "Copy settings from…",
    icon: "copy",
    group: "work",
    rank: 50,
    applies: (_, h) => has(h.copyAccountSettings),
    run: (a, h) => h.copyAccountSettings!(a),
  },
  {
    id: "default",
    label: (a) => (a.is_default ? "Stop being the default" : "Make the default"),
    icon: "default",
    group: "work",
    rank: 60,
    applies: (_, h) => has(h.toggleDefaultAccount),
    run: (a, h) => h.toggleDefaultAccount!(a),
  },
  { id: "remove", label: "Remove…", icon: "remove", group: "stop", rank: 10, applies: (_, h) => has(h.removeAccount), run: (a, h) => h.removeAccount!(a) },
];

export const TOOL: ActionSpec<"tool">[] = [
  { id: "open", label: "Open its page", icon: "page", group: "go", rank: 10, applies: (_, h) => has(h.openTool), run: (t, h) => h.openTool!(t) },
  { id: "accounts", label: "Its accounts", icon: "account", group: "go", rank: 20, applies: (_, h) => has(h.toolAccounts), run: (t, h) => h.toolAccounts!(t) },
  {
    id: "copy-update",
    label: "Copy update command",
    icon: "copy",
    group: "work",
    rank: 10,
    applies: (t, h) => has(h.copyUpdateCommand) && t.install_command !== null,
    run: (t, h) => h.copyUpdateCommand!(t),
  },
];

export const THIS_MAC: ActionSpec<"this-mac">[] = [
  {
    id: "open",
    label: "Open the session's page",
    icon: "page",
    group: "go",
    rank: 10,
    // A tool AI Juggler didn't start has no actions (D-08).
    applies: (tree, h) => has(h.openSessionById) && tree.session_id !== null,
    run: (tree, h) => h.openSessionById!(tree.session_id!),
  },
  {
    id: "not-ours",
    label: "Not started by AI Juggler",
    icon: "info",
    group: "go",
    rank: 20,
    applies: (tree) => tree.session_id === null,
    blocked: () => "AI Juggler only acts on sessions it started",
    run: () => {},
  },
];
