/**
 * D-62 decision 8 (AJ-E3008): where a pool works, set on its page:
 * every project, or the ones picked (chips; open projects first in the
 * picker). Agents see only the pool's projects; a broad member brings in
 * only its sessions there. A project's own pool keeps its project.
 */
import { projectChoices } from "../../lib/project-words";
import { useState, type JSX } from "react";

import { Modal } from "../../components/form";
import { Button } from "../../components/primitives";
import { Picker, Segmented } from "../../components/controls";
import { showToast } from "../../components/Toast";
import { errorText } from "../../lib/bridge";
import type { ProjectInfo } from "../../types/generated/ProjectInfo";
import { poolSetScope } from "../../lib/ipc/pool-model";
import type { PoolInfo } from "../../lib/ipc/pools";
import { setPools } from "../../lib/pools";

export function ScopeSheet({ pool, projects, open, onClose }: { pool: PoolInfo; projects: ProjectInfo[]; open: ReadonlySet<string>; onClose: () => void }): JSX.Element {
  const [every, setEvery] = useState(pool.scope_all);
  const [picked, setPicked] = useState<(string | null)[]>(pool.scope_all ? [] : pool.scope);
  const [busy, setBusy] = useState(false);
  const name = (id: string | null) => (id === null ? "No project" : (projects.find((one) => one.id === id)?.name ?? "A project"));
  const addable = projects.filter((one) => !picked.includes(one.id));
  const save = async () => {
    setBusy(true);
    try {
      setPools(await poolSetScope(pool.id, every, every ? [] : picked));
      onClose();
    } catch (cause) {
      showToast({ message: "Couldn't change where it works", detail: errorText(cause), tone: "error" });
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal title="Where it works" subtitle="Agents see only this pool's projects. An App, Account or Tool brings in only its sessions here." onClose={onClose} wide testId="scope-sheet">
      <div className="flex flex-col gap-3">
        <Segmented
          label="Where it works"
          value={every ? "every" : "these"}
          segments={[
            { value: "every", label: "Every project" },
            { value: "these", label: "These projects" },
          ]}
          onChange={(value) => setEvery(value === "every")}
          className="self-start"
          testId="scope-sheet-where"
        />
        {!every && (
          <div className="flex flex-wrap items-center gap-1.5">
            {picked.map((id) => (
              <span key={id ?? "none"} className="inline-flex items-center gap-1 rounded-full border border-hairline bg-surface-2 py-0.5 pl-2.5 pr-1 text-meta text-ink">
                {name(id)}
                <button type="button" aria-label={`Take ${name(id)} out`} className="rounded-full px-1 text-muted hover:text-ink" onClick={() => setPicked(picked.filter((one) => one !== id))}>
                  ×
                </button>
              </span>
            ))}
            <Picker
              ariaLabel="Add a project"
              heading="Add a project"
              value={null}
              size="sm"
              placeholder="+ Add a project"
              groups={[{ label: "Open now", values: addable.filter((one) => open.has(one.id)).map((one) => one.id) }]}
              allHeading="All projects"
              searchLabel="Search projects"
              options={[...projectChoices(addable), ...(picked.includes(null) ? [] : [{ value: "__none", label: "Sessions with no project" }])]}
              onChange={(value) => setPicked([...picked, value === "__none" ? null : value])}
              testId="scope-sheet-add"
            />
          </div>
        )}
        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" disabled={busy || (!every && picked.length === 0)} onClick={() => void save()} data-testid="scope-sheet-save">
            Save
          </Button>
        </div>
      </div>
    </Modal>
  );
}
