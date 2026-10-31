<!-- captures: e0eb9effb031 -->
# 12-model: what a session's model is: a model change seen in the transcript, a model the tool doesn't report, and the rail with and without the change mark

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

overview-drift-1440x900 states the change plainly in the Where it runs card: "Launched with opus · now running sonnet-5 (observed in transcript)" under "Model sonnet-5", which is easy to read. not-reported-1440x900 says "Model: not reported by Claude Code" and adds a quota box with a "Park until reset" button; the box's paragraph wraps into a narrow column beside the button. The same capture's terminal card says "Ended" while the meta line says "Running in AI Juggler · started 5h ago". rail-drift-1440x900 and rail-plain-1440x900 differ only by a tiny three-dot mark under one row's status icon, which is nearly invisible and has no words, so the rail barely shows the model change. All captures were re-rendered.

## Weak

- rail-drift-1440x900: the model-change mark is a tiny unlabelled dot glyph, hard to see next to rail-plain.
- not-reported-1440x900: the terminal says "Ended" while the meta line says "Running in AI Juggler".
