/**
 * AJ-E1507: a project's rules files and MCP servers across tools and
 * accounts. Only the `apply` calls write, and only on the user's click.
 * The preview and the tests read the seam `__AIJ_PROJECT_SYNC__` instead
 * of the backend; without it there's nothing to compare.
 */
import { hasLiveBackend } from "../bridge";
import { invoke } from "./transport";
import type { AppliedInfo } from "../../types/generated/AppliedInfo";
import type { DiffLineInfo } from "../../types/generated/DiffLineInfo";
import type { RulesDiffInfo } from "../../types/generated/RulesDiffInfo";
import type { RulesFileInfo } from "../../types/generated/RulesFileInfo";
import type { RulesSyncInfo } from "../../types/generated/RulesSyncInfo";
import type { ServerAccountInfo } from "../../types/generated/ServerAccountInfo";
import type { ServerCopyInfo } from "../../types/generated/ServerCopyInfo";
import type { ServersSyncInfo } from "../../types/generated/ServersSyncInfo";
import type { ProjectSyncSeam } from "../../preview/project-sync";

export type {
  AppliedInfo,
  DiffLineInfo,
  RulesDiffInfo,
  RulesFileInfo,
  RulesSyncInfo,
  ServerAccountInfo,
  ServerCopyInfo,
  ServersSyncInfo,
};

export type { ProjectSyncSeam };

const seam = () => (window as unknown as { __AIJ_PROJECT_SYNC__?: ProjectSyncSeam }).__AIJ_PROJECT_SYNC__;

const NOTHING: RulesSyncInfo = { master: null, files: [], in_sync: true, summary: null };
const NO_SERVERS: ServersSyncInfo = { accounts: [], copies: [] };

/** The project's rules files and how each stands beside the master
 * (`master`, or the newest file with its own text). Reads only. */
export async function projectSyncRules(projectId: string, master: string | null = null): Promise<RulesSyncInfo> {
  if (!hasLiveBackend()) {
    const rules = seam()?.rules ?? NOTHING;
    if (!master || master === rules.master) return rules;
    return {
      ...rules,
      master,
      files: rules.files.map((file) => ({
        ...file,
        standing: file.name === master ? "master" : file.standing === "master" ? "differs" : file.standing,
        words: file.name === master ? "The master" : `Differs from ${master}`,
        can_apply: file.name !== master,
        counts: file.name !== master,
      })),
    };
  }
  return invoke<RulesSyncInfo>("project_sync_rules", { projectId, master });
}

/** What applying the master to `file` would change (`how`: `copy` the
 * master's text, or `read` it through the tool's import line). Reads
 * only. */
export async function projectSyncDiff(
  projectId: string,
  master: string,
  file: string,
  how: "copy" | "read",
): Promise<RulesDiffInfo> {
  if (!hasLiveBackend()) {
    const found = seam()?.diffs?.[`${file}|${how}`];
    return found ?? { file, master, how, groups: [], added: 0, removed: 0 };
  }
  return invoke<RulesDiffInfo>("project_sync_diff", { projectId, master, file, how });
}

/** Applies the master to `file`, against exactly what was shown (the
 * files' hashes). */
export async function projectSyncApply(
  projectId: string,
  master: string,
  file: string,
  how: "copy" | "read",
  fileSha: string | null,
  masterSha: string,
): Promise<AppliedInfo> {
  if (!hasLiveBackend()) {
    const current = seam();
    if (current?.rules) {
      const files = current.rules.files.map((candidate) =>
        candidate.name === file
          ? { ...candidate, standing: how === "read" ? "reads" : "same", words: `${how === "read" ? "Reads" : "Same as"} ${master}`, counts: false }
          : candidate,
      );
      current.rules = { ...current.rules, files, in_sync: true, summary: "Rules in sync" };
    }
    const message = how === "read" ? `${file} now reads ${master}` : `${file} now has the rules from ${master}`;
    return { offer: file, record: `record-${file}`, message };
  }
  return invoke<AppliedInfo>("project_sync_apply", { projectId, master, file, how, fileSha, masterSha });
}

/** Each account's MCP servers for the project, and what one could get
 * from another. Reads only. */
export async function projectSyncServers(projectId: string): Promise<ServersSyncInfo> {
  if (!hasLiveBackend()) return seam()?.servers ?? NO_SERVERS;
  return invoke<ServersSyncInfo>("project_sync_servers", { projectId });
}

/** Copies one MCP server from one account to another. */
export async function projectSyncServersApply(
  projectId: string,
  copy: Pick<ServerCopyInfo, "from" | "to" | "offer">,
): Promise<AppliedInfo> {
  if (!hasLiveBackend()) {
    const current = seam();
    const found = current?.servers?.copies.find((candidate) => candidate.offer === copy.offer && candidate.to === copy.to);
    if (current?.servers && found) {
      current.servers = {
        accounts: current.servers.accounts.map((account) =>
          account.account === copy.to ? { ...account, servers: [...account.servers, found.server].sort() } : account,
        ),
        copies: current.servers.copies.filter((candidate) => candidate !== found),
      };
    }
    return {
      offer: copy.offer,
      record: `record-${copy.offer}`,
      message: found ? `Set up the MCP server ${found.server}` : "It isn't on that account any more",
    };
  }
  return invoke<AppliedInfo>("project_sync_servers_apply", { projectId, from: copy.from, to: copy.to, offer: copy.offer });
}
