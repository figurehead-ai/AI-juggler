import { useEffect, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Button } from "./primitives";
import { Field, Input, Modal } from "./form";
import { MenuItem, RowMenu } from "./RowMenu";
import {
  errorText,
  type InstanceInfo,
  type ProjectInfo,
} from "../lib/bridge";
import { editorChoices, projectForget, projectOpen, projectRelocate, projectRename, projectSetPinned, sessionRename, sessionSetPinned } from "../lib/ipc/manage";

/**
 * AJ-E1502: naming and tidying sessions and projects, in one place so
 * the rail's row menus and the project view offer the same actions in
 * the same words (D-25). Each hook returns the actions, a menu opener,
 * and the sheets to render.
 */

/** A session's title on screen: the user's label, else its branch, else
 * its folder (the rail's rule). */
export function sessionTitle(session: InstanceInfo): string {
  return session.label ?? ownTitle(session);
}

/** The title a session has without a label. */
export function ownTitle(session: InstanceInfo): string {
  const cwd = session.cwd ?? "";
  const folder = cwd.replace(/\/+$/, "").split("/").pop() || cwd || session.account_label;
  return session.git_branch ?? folder;
}

/** The editor projects open in (Settings), or null for Finder. */
export function useEditorName(): string | null {
  const [name, setName] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    const load = () =>
      void editorChoices().then(
        (choices) => alive && setName(choices.chosen),
        () => null,
      );
    load();
    window.addEventListener(EDITOR_EVENT, load);
    return () => {
      alive = false;
      window.removeEventListener(EDITOR_EVENT, load);
    };
  }, []);
  return name;
}

/** Settings fires this when the editor choice changes. */
export const EDITOR_EVENT = "aij:editor-changed";

/** "Open in Zed", or "Open in Finder" when no editor is chosen. */
export function openWords(editor: string | null): string {
  return `Open in ${editor ?? "Finder"}`;
}

/** One rename sheet for sessions and projects. */
export function RenameSheet({
  title,
  initial,
  placeholder,
  hint,
  testId,
  onSave,
  onClose,
}: {
  title: string;
  initial: string;
  placeholder: string;
  hint: string;
  testId: string;
  onSave: (name: string) => Promise<void>;
  onClose: () => void;
}) {
  const [name, setName] = useState(initial);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  return (
    <Modal title={title} onClose={onClose}>
      <form
        className="flex flex-col gap-4"
        data-testid={testId}
        onSubmit={async (event) => {
          event.preventDefault();
          setBusy(true);
          setError(null);
          try {
            await onSave(name);
          } catch (cause) {
            setError(errorText(cause));
            setBusy(false);
          }
        }}
      >
        <Field label="Name" hint={hint}>
          <Input autoFocus value={name} placeholder={placeholder} onChange={(event) => setName(event.target.value)} data-testid={`${testId}-name`} maxLength={80} />
        </Field>
        {error && (
          <p className="text-xs text-danger" role="alert">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button size="sm" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button size="sm" variant="primary" type="submit" disabled={busy} data-testid={`${testId}-save`}>
            {busy ? "Saving…" : "Save"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}

/** A refusal or failure from an action, said in the app's words. */
function ProblemSheet({ title, message, onClose }: { title: string; message: string; onClose: () => void }) {
  return (
    <Modal title={title} onClose={onClose}>
      <div className="flex flex-col gap-4 text-sm">
        <p className="leading-relaxed text-muted" role="alert" data-testid="manage-problem">
          {message}
        </p>
        <div className="flex justify-end">
          <Button size="sm" variant="secondary" onClick={onClose} autoFocus>
            OK
          </Button>
        </div>
      </div>
    </Modal>
  );
}

/** The project actions (rail menu and project view): rename, pin, open
 * in the editor or Finder, relocate, forget. */
export function useProjectManage(onChanged: () => void) {
  const editor = useEditorName();
  const [menu, setMenu] = useState<{ project: ProjectInfo; at: { x: number; y: number } } | null>(null);
  const [sheet, setSheet] = useState<{ kind: "rename" | "forget"; project: ProjectInfo } | null>(null);
  const [problem, setProblem] = useState<{ title: string; message: string } | null>(null);
  const run = async (title: string, action: () => Promise<boolean | void>) => {
    try {
      if ((await action()) !== false) onChanged();
    } catch (cause) {
      setProblem({ title, message: errorText(cause) });
    }
  };
  const actions = {
    rename: (project: ProjectInfo) => setSheet({ kind: "rename", project }),
    togglePin: (project: ProjectInfo) => run(`Couldn't pin ${project.name}`, () => projectSetPinned(project.id, !project.pinned)),
    open: (project: ProjectInfo) => run(`Couldn't open ${project.name}`, () => projectOpen(project.id, "editor")),
    reveal: (project: ProjectInfo) => run(`Couldn't show ${project.name}`, () => projectOpen(project.id, "finder")),
    relocate: (project: ProjectInfo) =>
      run(`Couldn't relocate ${project.name}`, () => projectRelocate(project.id)),
    forget: (project: ProjectInfo) => setSheet({ kind: "forget", project }),
  };
  const choose = (action: (project: ProjectInfo) => unknown) => () => {
    const project = menu?.project;
    setMenu(null);
    if (project) void action(project);
  };
  const missing = menu?.project.missing === true;
  const gone = missing ? "Its folder isn't there any more" : undefined;
  // Sheets go on the page itself (a blurred rail would clip them).
  const element: ReactNode = createPortal(
    <>
      {menu && (
        <RowMenu heading={menu.project.name} ariaLabel={`Actions for ${menu.project.name}`} at={menu.at} onClose={() => setMenu(null)}>
          <MenuItem testId="project-rename" onClick={choose(actions.rename)}>
            Rename…
          </MenuItem>
          <MenuItem testId="project-pin" onClick={choose(actions.togglePin)}>
            {menu.project.pinned ? "Unpin" : "Pin to top"}
          </MenuItem>
          <MenuItem testId="project-open" disabled={missing} title={gone} onClick={choose(actions.open)}>
            {openWords(editor)}
          </MenuItem>
          {editor !== null && (
            <MenuItem testId="project-reveal" disabled={missing} title={gone} onClick={choose(actions.reveal)}>
              Show in Finder
            </MenuItem>
          )}
          <MenuItem testId="project-relocate" onClick={choose(actions.relocate)}>
            Relocate…
          </MenuItem>
          <MenuItem testId="project-forget" danger onClick={choose(actions.forget)}>
            Forget…
          </MenuItem>
        </RowMenu>
      )}
      {sheet?.kind === "rename" && (
        <RenameSheet
          title={`Rename ${sheet.project.name}`}
          initial={sheet.project.name}
          placeholder={sheet.project.root.replace(/\/+$/, "").split("/").pop() ?? ""}
          hint="A name in AI Juggler only: the folder keeps its own. Leave it empty to use the folder's name."
          testId="rename-project"
          onClose={() => setSheet(null)}
          onSave={async (name) => {
            await projectRename(sheet.project.id, name);
            setSheet(null);
            onChanged();
          }}
        />
      )}
      {sheet?.kind === "forget" && (
        <ForgetSheet
          project={sheet.project}
          onClose={() => setSheet(null)}
          onDone={() => {
            setSheet(null);
            onChanged();
          }}
        />
      )}
      {problem && <ProblemSheet title={problem.title} message={problem.message} onClose={() => setProblem(null)} />}
    </>,
    document.body,
  );
  return {
    editor,
    actions,
    openMenu: (project: ProjectInfo, at: { x: number; y: number }) => setMenu({ project, at }),
    menuOpenFor: menu?.project.id ?? null,
    element,
  };
}

function ForgetSheet({ project, onClose, onDone }: { project: ProjectInfo; onClose: () => void; onDone: () => void }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  return (
    <Modal title={`Forget ${project.name}?`} onClose={onClose}>
      <div className="flex flex-col gap-4 text-sm" data-testid="forget-project">
        <p className="leading-relaxed text-muted">
          It leaves the rail and New session until you start a session there again. Nothing in the folder changes, and its sessions stay in History.
        </p>
        {error && (
          <p className="text-xs text-danger" role="alert">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button size="sm" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button
            size="sm"
            variant="danger"
            disabled={busy}
            data-testid="forget-project-confirm"
            onClick={async () => {
              setBusy(true);
              setError(null);
              try {
                await projectForget(project.id, true);
                onDone();
              } catch (cause) {
                setError(errorText(cause));
                setBusy(false);
              }
            }}
          >
            {busy ? "Forgetting…" : "Forget"}
          </Button>
        </div>
      </div>
    </Modal>
  );
}

/** The session actions (rail menu and session view): rename, pin. */
export function useSessionManage(onChanged: () => void) {
  const [renaming, setRenaming] = useState<InstanceInfo | null>(null);
  const [problem, setProblem] = useState<{ title: string; message: string } | null>(null);
  const actions = {
    rename: (session: InstanceInfo) => setRenaming(session),
    togglePin: async (session: InstanceInfo) => {
      try {
        await sessionSetPinned(session.id, !session.pinned);
        onChanged();
      } catch (cause) {
        setProblem({ title: `Couldn't pin ${sessionTitle(session)}`, message: errorText(cause) });
      }
    },
  };
  const element: ReactNode = createPortal(
    <>
      {renaming && (
        <RenameSheet
          title={`Rename ${sessionTitle(renaming)}`}
          initial={renaming.label ?? ""}
          placeholder={ownTitle(renaming)}
          hint="Shown over the session's own title. Leave it empty to use the title again."
          testId="rename-session"
          onClose={() => setRenaming(null)}
          onSave={async (name) => {
            await sessionRename(renaming.id, name.trim() === "" ? null : name);
            setRenaming(null);
            onChanged();
          }}
        />
      )}
      {problem && <ProblemSheet title={problem.title} message={problem.message} onClose={() => setProblem(null)} />}
    </>,
    document.body,
  );
  return { actions, element };
}
