/**
 * AJ-E1502 — Settings › General: the editor projects open in.
 */
import { useEffect, useState, type JSX } from "react";
import { EDITOR_EVENT } from "./ManageWork";
import { Card } from "./primitives";
import { errorText } from "../lib/bridge";
import { editorChoices, editorSet } from "../lib/ipc/manage";
import type { EditorChoicesInfo } from "../types/generated/EditorChoicesInfo";
import { CardTitle } from "./CardTitle";
import { Picker, RowChip } from "./controls";
import { IconEditor, IconFolderOpen } from "./icons-actions";

/** AJ-E1502: the editor projects open in ("Open in …" in a project's
 * menu); Finder when none is chosen. Only editors this Mac has show. */
export function EditorCard(): JSX.Element {
  const [choices, setChoices] = useState<EditorChoicesInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    void editorChoices().then(
      (found) => alive && setChoices(found),
      () => alive && setChoices({ installed: [], chosen: null }),
    );
    return () => {
      alive = false;
    };
  }, []);
  return (
    <Card className="flex flex-col gap-3 p-5" data-testid="settings-editor">
      <CardTitle id="editor-title">
        Open projects in
      </CardTitle>
      {/* AJ-E1558 (D-126): the row's chip, then the choice, each option
          with its icon (Finder's folder, an editor's). */}
      <div className="flex items-center gap-3">
      <RowChip icon={choices?.chosen ? IconEditor : IconFolderOpen} on={Boolean(choices?.chosen)} />
      <Picker choice
        ariaLabel="Open projects in"
        className="max-w-xs self-start"
        disabled={choices === null}
        value={choices?.chosen ?? ""}
        testId="settings-editor-choice"
        onChange={async (value) => {
          const next = value === "" ? null : value;
          setError(null);
          try {
            await editorSet(next);
            setChoices((current) => (current ? { ...current, chosen: next } : current));
            window.dispatchEvent(new Event(EDITOR_EVENT));
          } catch (cause) {
            setError(errorText(cause));
          }
        }}
        options={[{ value: "", label: "Finder", icon: IconFolderOpen }, ...(choices?.installed ?? []).map((name) => ({ value: name, label: name, icon: IconEditor }))]}
      />
      </div>
      <p className="text-label text-muted">
        {choices !== null && choices.installed.length === 0
          ? "No editor AI Juggler knows is installed, so projects open in Finder."
          : "A project's menu opens it here. Show in Finder is always there too."}
      </p>
      {error && (
        <p className="text-label text-danger" role="alert">
          {error}
        </p>
      )}
    </Card>
  );
}
