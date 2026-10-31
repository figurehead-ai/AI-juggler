<!-- captures: f6b328288a47 -->
# 12-model: what a session's model is: a model change seen in the transcript, a model the tool doesn't report, and the rail with and without the change mark

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

overview-drift-1440x900 states the change plainly in the Where it runs card: "Launched with opus · now running sonnet-5 (observed in transcript)" under "Model sonnet-5", wrapped onto two lines inside the card, which is easy to read. not-reported-1440x900 says "Model: not reported by Claude Code" and adds a quota box with a "Park until reset" button; the box's paragraph still wraps into a narrow six-line column beside the button. The same capture's terminal card says "Ended" while the meta line says "Running in AI Juggler · started 5h ago". rail-drift-1440x900 and rail-plain-1440x900 still differ only by a tiny "…" mark under the feat/read-model error row's status icon, which is nearly invisible and has no words, so the rail barely shows the model change. The design fixes show as the lighter accent on the logo mark, the selected tab, "Open its page" and the runway bar, a one-row top bar, and rail error rows that read "Stopped with an error" with no cut age. All captures were re-rendered. The second pass shows only in the two overview captures: the terminal's black now runs to its card's edges, with black padding before the text instead of a lighter frame.

## Weak

- rail-drift-1440x900: the model-change mark is a tiny unlabelled dot glyph, hard to see next to rail-plain.
- not-reported-1440x900: the terminal says "Ended" while the meta line says "Running in AI Juggler".
- not-reported-1440x900: the quota box's paragraph wraps into a narrow column beside "Park until reset".
