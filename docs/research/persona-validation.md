# Persona-based validation by an AI tester: the research (AJ-E3307)

Prepared 2026-11-02 for 0.4's pre-announcement validation; the protocol AI Juggler follows is [`docs/qa/persona-validation.md`](../qa/persona-validation.md), which adapts §6 below. Scope: how an LLM agent that drives a desktop app through screenshots, clicks and typing can run a persona-based usability study the way a UX researcher runs moderated tests, and produce results that are trustworthy, evidence-based and ranked.

**Bottom line up front.** The evidence supports using LLM agents to *find and rank candidate problems* cheaply and repeatably. It does not support treating their *satisfaction scores* as a stand-in for real users' scores. Every study that compared simulated users with real people found the simulated ones more successful, more goal-directed, more positive and less varied than humans. So the protocol below is built to correct for that. A fresh tester has no product knowledge. It must state expectations before acting. Confusion is measured from behaviour, not only from self-ratings. A separate critic checks the tester's claims against screenshots. Thresholds are set stricter than human norms. The maintainer's own dogfooding is the calibration anchor.

---

## 1. Human usability methods worth borrowing, and how to adapt them

| Method | What it gives a human researcher | Adaptation for an agent tester |
|---|---|---|
| **Task-based moderated testing** | Realistic goals, observed behaviour, success/failure | Tasks phrased as the persona's goal in their own words ("You're about to hit your limit on the main account; keep this chat going on another one"), never as UI instructions ("click Switch"). The orchestrator acts as moderator: it gives the task, does not help, and records. |
| **Think-aloud** | *Why* users struggle; misconceptions ([NN/g](https://www.nngroup.com/articles/thinking-aloud-the-1-usability-tool/)) | It comes free with an LLM, but the risk is fluent post-hoc rationalisation. Make it structured: before each action the agent writes what it is looking for and what it expects. After the action it writes what it sees. NN/g's warning about facilitators who lead participants applies to the prompt too: the prompt must not hint at the path. |
| **Cognitive walkthrough** | Learnability defects at each step. Four questions: *Will users try to achieve the right result? Will they notice the correct action is available? Will they associate the action with their goal? After the action, will they see progress?* A "no" to any of them fails the step ([NN/g](https://www.nngroup.com/articles/cognitive-walkthroughs/)) | This is the best fit for agents. Use the four questions as the per-step record. Synthetic cognitive-walkthrough work found that the agent should list and rate *all plausible next actions* before choosing, and that a separate confusion-rating pass over the full history predicted human failure points best ([arXiv 2512.03568](https://arxiv.org/html/2512.03568v1)). |
| **Heuristic evaluation (Nielsen's 10)** | Broad sweep for standard violations ([NN/g](https://www.nngroup.com/articles/ten-usability-heuristics/)) | Run it as a *separate* pass on captured screens, not mixed into the persona walk. LLMs did well here, but with many false positives and weak cross-screen consistency checking (§2). |
| **Severity ratings 0–4** | 0 not a problem, 1 cosmetic, 2 minor, 3 major ("important to fix"), 4 catastrophe ("imperative to fix before release"). Severity combines frequency, impact and persistence, plus market impact. One evaluator's ratings are "too unreliable"; the mean of three is adequate ([NN/g](https://www.nngroup.com/articles/how-to-rate-the-severity-of-usability-problems/)) | Use the scale as it stands. Get three independent ratings (tester run, critic, builder) and take the mean. |
| **Task success** | Binary or graded completion | Grade it: *success*, *success with difficulty* (backtracking, or a wrong first click), *failed* (wrong end state that the agent believed was right), *gave up*. The "believed it succeeded but didn't" case matters most for honesty and must be checked against app state. |
| **Time on task, errors** | Efficiency; wrong clicks, backtracks | Agent wall-clock time is mostly model latency, so record **app response time per action**, **steps vs. shortest path**, wrong clicks, backtracks and loops instead. These behavioural measures resist positivity bias. |
| **SEQ** (1–7, "Overall, how difficult or easy was the task to complete?") | Post-task ease. Human average is about 5.3–5.6 across 400+ tasks. Correlates with time and success at about r = .5. Ask why for ratings below 5 ([MeasuringU](https://measuringu.com/seq10/)) | Ask it after every task, with a written reason. Expect inflation (§2), so read it alongside behaviour. |
| **SUS / UMUX-Lite** | Post-study perceived usability. SUS average is 68; on the Sauro–Lewis curve, A is about 80.8+ and promoters average about 81 ([MeasuringU](https://measuringu.com/interpret-sus-score/)). UMUX-Lite is two 7-point items ("capabilities meet my requirements", "is easy to use") and correlates r = .83 with SUS ([MeasuringU](https://measuringu.com/umux-lite/)) | Use UMUX-Lite per persona at the end. It is short, and its "capabilities meet my requirements" item captures *value*, which SUS does not. Note that MeasuringU now prefers linear interpolation over the old UMUX-Lite→SUS regression, which underestimates high scores ([MeasuringU](https://measuringu.com/from-umux-lite-to-ux-lite/)). |
| **NPS / likelihood to recommend** | Loyalty proxy | Record it, but give it little weight. It is the score most exposed to sycophancy (§2). |
| **First-click testing** | When the first click is right, about 87% eventually succeed; when it is wrong, about 46% do (Bailey & Wolfson). MeasuringU later found 81% vs. 14% in tree tests but cautions that this is not the same as live-site success ([MeasuringU](https://measuringu.com/do-click-tests-predict-live-site-clicks/)) | Cheap and agent-friendly. Show a fresh agent a static screenshot plus the goal and ask "where would you click first?" before it may explore. Record whether the first click was right. |
| **Five-second test** | First-impression comprehension only, not usability ([Smashing case study](https://smashingmagazine.com/2023/12/five-second-testing-case-study)) | Show a fresh agent the first-run screen and the main window once, then ask: what is this app, what is it telling you, what would you do first? An agent cannot really be limited to "five seconds", so treat this as a *clarity test*. |
| **Desirability / reaction words** | How it makes people feel: pick about 5 words from a list, then explain. Qualitative only; it doesn't measure levels ([NN/g](https://www.nngroup.com/articles/desirability-reaction-words/), [MeasuringU](https://measuringu.com/?p=578)) | Covers the "how would it make the user feel" dimension. Use a shortened, balanced list with half negative words, so the agent has to pick. |

**Comparable scales.** Keep the standard published anchors (SEQ 1–7, UMUX-Lite 1–7, severity 0–4) so results can be read against published human norms, even though agent scores will skew high. Avoid inventing 0–100 sliders: there are no norms for them. One study also reports agent rating baselines diverging most from humans' on broad 0–100 scales. That comes from a secondary summary I could not verify, so it is unconfirmed.

**How many sessions.** Nielsen's model (problems found ≈ 1 − (1 − 0.31)^n) gives about 85% with five similar users. For distinct groups Nielsen recommends 3–4 users per group ([NN/g](https://www.nngroup.com/articles/why-you-only-need-to-test-with-5-users/)). That model assumes independent humans. Agent runs are *not* independent, because they share one model's blind spots, so extra runs add less than extra humans would (§2).

---

## 2. What is known about LLM agents as simulated users and usability testers

### Systems and studies

- **UXAgent** (Northeastern/Amazon, CHI EA '25 and later versions) has a persona generator, a two-loop agent (fast perceive–plan–act, slow reflect/"wonder"), memory, a replay UI and post-session agent interviews. Sixteen UX researchers reviewed 20 agent sessions. They found it easy and helpful, but their trust was low (mean ≈ 0.57 on a bipolar scale), and they leaned against agents replacing humans (≈ −0.57). The paper has **no quantitative comparison with real participants** ([arXiv 2504.09407](https://arxiv.org/html/2504.09407v3)).
- **Agent A/B** ran 1,000 persona agents on Amazon.com. The result matched a parallel human test *directionally*. However, humans took about 16 actions per session against about 6 for agents: agents were "more goal-directed" and "simplify exploratory behavior." The authors say it is "not a replacement for real user testing" ([arXiv 2504.09723](https://arxiv.org/html/2504.09723v4)).
- **Synthetic cognitive walkthrough** compared GPT-4 and Gemini 2.5 Pro with 10 humans on two mobile apps. Task completion was 100% / 97% for the LLMs against 88% for humans. LLMs took about 7.5 steps against about 10.4 and diverged far less from the ideal path. Humans flagged about 7.5 potential failure points each; LLM runs flagged about 0.5. Humans explored breadth-first, while LLMs "consistently committed to the most probable option" even when they themselves noted ties. A separate with-context confusion-rating pass predicted human failure points (odds ratio 5.5–7.5 for GPT), with run-to-run Cohen's κ ≈ 0.63–0.64 ([arXiv 2512.03568](https://arxiv.org/html/2512.03568v1)).
- **Synthetic heuristic evaluation** compared GPT-4, Gemini and Claude 3.5 Sonnet with 5 human experts on screenshots of two apps. GPT-4 covered 73–77% of the master issue set, against 17–18% for an average single expert and 57–63% for five experts together. GPT-4 was strongest on *aesthetic and minimalist design* and weaker than experts on *user control and freedom* and *cross-screen consistency* (43–50% against 83–86%). It produced many severity-0 non-issues: it misread OS icons, took banners for buttons and missed platform conventions. It also produced duplicates. GPT-4 re-detected 94–96% of its own issues across runs over months, which means it was **stable, including in its blind spots** ([arXiv 2507.02306](https://arxiv.org/html/2507.02306v1)).
- **GPT-4o heuristic evaluation vs. HCI experts** found that only 21.2% of the experts' issues were also found by GPT-4o. GPT-4o added 27 issues the experts did not report, some of them hallucinated ([arXiv 2506.16345](https://arxiv.org/abs/2506.16345v1)).
- **UICrit** collected 3,059 designer critiques of 983 UIs. Few-shot examples plus visual prompting improved LLM critique quality by about 55% (0.31 → 0.48), still short of human designers at 0.75. Few-shot examples also appeared to reduce hallucination ([arXiv 2407.08850](https://arxiv.org/abs/2407.08850)).
- **uxCUA** (2026): off-the-shelf computer-use agents "still give inaccurate usability assessments", so the authors train a dedicated one ([arXiv 2604.26020](https://arxiv.org/abs/2604.26020)).
- **Agent-based GUI testing for functional bugs** works well. GPTDroid reached 75% activity coverage and found 31% more bugs than baselines on 93 apps ([arXiv 2310.15780](https://arxiv.org/abs/2310.15780)). Bug finding is the most solid use of the persona walk.

### Agreement with real users: the candid version

- **Problem discovery.** There is partial, non-trivial overlap: 21% of expert issues in one study, 73–77% of a pooled set in another, plus many false positives. Agents beat a *single* human evaluator on coverage of static issues and lose on interaction, recovery and cross-screen issues.
- **Behaviour.** Agents succeed more, explore less and get lost less (Synthetic CW, Agent A/B). They therefore **under-detect the very confusion you care about** unless that confusion is measured separately.
- **Attitudes and ratings.** NN/g reran three of its real studies with synthetic users. The synthetic users were overly positive (one said it completed all its courses; the real user finished 3 of 7), "care about everything", and were "one-dimensional" and sycophantic ([NN/g, 2024](https://www.nngroup.com/articles/synthetic-users/)). A 451-person τ-bench study found LLM simulators "excessively cooperative", giving "uniformly more positive feedback" and lacking "realistic frustration" ([arXiv 2603.11245](https://arxiv.org/abs/2603.11245)). Synthetic consumer panels reproduce direction but compress variance and miss effect sizes ([NN/g, 2025](https://www.nngroup.com/articles/ai-simulations-studies/)). Persona prompts also portray groups as outsiders see them and flatten diversity ([Wang et al., arXiv 2402.01908](https://arxiv.org/abs/2402.01908)). The best fidelity found anywhere comes from agents seeded with two-hour interviews of *specific real people*: 0.85 normalised GSS accuracy, versus lower scores for demographic or persona-only prompts ([Park et al., arXiv 2411.10109](https://arxiv.org/abs/2411.10109v1)). A persona paragraph is the weak case.
- **Context matters: humans disagree too.** Across evaluators, only 20% of 93 problems were found by all four and 46% by just one; top-10 severity lists did not overlap ([MeasuringU on Hertzum & Jacobsen](https://measuringu.com/evaluator-effect/)). Agent-vs-human disagreement has to be judged against that human-vs-human baseline. It is not a reason to discard agent findings, but it is a reason never to trust a single run.

**There is no published evidence that an LLM agent's SEQ, SUS, UMUX-Lite or NPS for a product matches what real users would give.** All adjacent evidence points to inflation.

### Failure modes (with source)

1. **Over-positivity and sycophancy.** Preference training rewards agreeable answers ([Sharma et al.](https://arxiv.org/abs/2310.13548)); simulated users rate higher (NN/g; arXiv 2603.11245).
2. **Self-preference.** LLM judges favour their own outputs ([Panickssery et al.](https://arxiv.org/abs/2404.13076)). This is the curse of knowledge in machine form when the tester also built the UI ("You ≠ User", [NN/g](https://www.nngroup.com/videos/you-are-not-user-slogan/)).
3. **Knowledge leakage.** It knows internal names and intended flows.
4. **Over-rational navigation.** It rarely gets lost and has perfect memory (Synthetic CW; Agent A/B).
5. **Perceptual and literal errors.** It misreads icons, OS chrome and conventions (Synthetic HE). Its own GUI skill is limited: on OSWorld, humans scored about 72% and the best agent at release about 12% ([summary](https://www.emergentmind.com/topics/osworld)). So some "failures" belong to the agent, not the UI.
6. **Weak cross-screen, recovery and undo judgment.** Clutter and minimalism are handled fine (Synthetic HE).
7. **Moderate run-to-run agreement** (κ ≈ 0.63), with blind spots shared across runs of one model.
8. **Flattened, stereotyped personas** that want everything.

---

## 3. Mitigations, especially for the curse of knowledge

Ordered by how much they buy:

1. **Separate, fresh tester agent with an information firewall.** It gets only the persona brief, the task goal, and live screenshots and input. It has no repo, docs, design notes, memory files or previous transcripts. Run it in a clean working directory with tools limited to driving the GUI. This is the single biggest lever against leakage and self-preference: the evaluator never saw the generation.
2. **Leak lint.** The builder keeps a list of internal terms that never appear in the UI (code names, ADR and epic IDs, internal concept names). After each run, grep the tester's transcript for them. Any hit invalidates the affected steps. Also flag naming a control before it was visible.
3. **Predict, act, observe at every step.** These are the cognitive-walkthrough questions as fields. *Goal now; what I'm looking for; candidate controls I can see, each rated for how well it matches the goal; my pick and why; what I expect to happen*, then *what actually happened; did it match; did I see progress*. A mismatch is a finding even if the task finally succeeds. "Two or more candidates looked equally plausible" is logged as a confusion signal. This counters over-rationality ([arXiv 2512.03568](https://arxiv.org/html/2512.03568v1)).
4. **Measure confusion from behaviour, not only self-report.** Count wrong first clicks, backtracks, ties, hovers to discover meaning, and steps over the shortest path. These resist positivity bias.
5. **Separate confusion-rating pass with full context.** After the walk, a second call rates each screen 0/1/2 for "would a newcomer be confused here", given the whole history. This predicted human failure points best in the Synthetic CW study.
6. **Anchored rubrics with worked examples.** Each score level gets a written behavioural anchor and one example, drawn where possible from the maintainer's real past complaints. Rubric-plus-reference evaluation correlated strongly with humans in LLM-judge research ([Prometheus, r ≈ 0.9](https://arxiv.org/abs/2310.08491v2)). Few-shot examples improved UI critique and reduced hallucination ([UICrit](https://arxiv.org/abs/2407.08850)).
7. **Adversarial persona framing.** Brief the tester that the persona is busy, sceptical, has alternatives (terminal, tool UIs) and is evaluating whether to keep the app. Give it explicit permission and instructions to be dissatisfied, to give up after N failed attempts, and to say "I don't get it". That counters the "excessively cooperative" default ([arXiv 2603.11245](https://arxiv.org/abs/2603.11245)).
8. **Multiple independent runs.** Run each persona–flow three times with fresh contexts and varied wording of the brief. Report agreement, and treat an issue seen in 1 of 3 runs as a candidate and in 2 or more of 3 as confirmed. Where possible, add one run on a *different* model family, because single-model runs share blind spots ([arXiv 2507.02306](https://arxiv.org/html/2507.02306v1)).
9. **Critic pass, also fresh.** A separate agent receives the log and screenshots. It (a) verifies each claim against the pixels; (b) flags non-issues such as misread OS chrome or conventions; (c) classifies each failure as UI, agent-limitation or environment; (d) challenges any score of 6–7 that lacks a cited behaviour. 
10. **Builder triage last.** The builder, who has full code access, reproduces bugs and fixes them, but **may not lower a severity or score on its own**. Disagreements are written down and go to the maintainer.
11. **Human calibration anchor.** The maintainer *is* the primary persona (the multi-subscription developer). Have the maintainer do 2–3 of the same tasks with SEQ and one sentence each. Compare that with the agent runs and report the gap; it becomes the correction factor for this product. The maintainer's past dogfooding reports double as a recall check.
12. **Score stricter than human norms.** Inflation is documented, so thresholds in §6 sit above human averages.

---

## 4. Persona construction

**Evidence base.** NN/g distinguishes *proto-personas* (built from team assumptions, which risk becoming an "echo chamber"), *qualitative personas* (built from interviews with 5–30 users, the recommended kind) and *statistical personas* ([NN/g](https://www.nngroup.com/articles/persona-types/)). Before an announcement you cannot afford a full interview study. You can still ground every attribute in an observable source, and label anything else as an assumption:

- the maintainer's own usage and bug reports, the strongest source;
- public posts by developers juggling several AI coding subscriptions (rate-limit complaints, multi-account workarounds, profile setups), GitHub issues and discussions;
- how competing or adjacent tools describe their users;
- the product spec, only to *check coverage*.

**Persona template** (one page, with no features in it):

- *Who and context*: role, setup (tools, subscriptions, profiles, machines), working hours, team or solo.
- *Goals* (outcomes, not features): "never lose a session's context when a limit hits", "know at a glance which account still has headroom".
- *Daily triggers*: the moments that bring them to the app, such as morning start, hitting a limit mid-task, starting a new feature, a client switch, end of day.
- *Skill and mental model*: what they already know (terminals, Claude Code) and what vocabulary they use.
- *Frustrations and anti-goals*: what makes them abandon a tool, such as setup friction, anything that touches credentials, noise, or a hijacked focus.
- *Alternatives*: what they do today without the app. The app must beat this.
- *Source* for each line, with an `[assumption]` tag where there is none.

**Keeping personas from becoming wish lists.** Personas contain no feature names and no "wants X". Each needs at least two frustrations, one thing they *don't* care about, and a concrete alternative they would fall back to. Write them *before* looking at the feature list, then map flows to features. That way, features with no persona trigger show up as candidates for being hidden (progressive disclosure), not as missing tests.

**From personas to flows.** For each trigger, write one scenario: starting state, goal in the persona's words, success criterion as an observable end state, and an "abandon if" condition. Start every persona with **first run**: install/open, import, first view of the main window, plus a five-second test. Then cover the daily triggers in a realistic order across a simulated day.

**How many.** Three personas, matching the brief: the solo multi-subscription developer, the team lead, and the freelancer with client accounts. Each gets 4–6 flows, about 15 flows in total. That follows NN/g's 3–6 personas and Nielsen's ≥ 3 sessions per distinct group (three runs each). Weight the primary persona about 2×, because it is the launch audience.

---

## 5. Evidence and reporting

**Per-step record** (one JSON or Markdown row per action):

| Field | Content |
|---|---|
| ids | run, persona, flow, step, timestamp |
| before | screenshot path |
| intent | goal now, and what the tester is looking for |
| candidates | visible controls considered, rated 0–2 for fit; tie flag |
| expectation | what should happen |
| action | click/type/key + target (label as seen on screen) + coordinates |
| after | screenshot path; app response time (ms) from action to stable screen |
| observation | what changed; expectation met yes/partly/no |
| CW verdict | the four questions, Y/N each |
| notes | verbatim think-aloud, ≤ 2 sentences |

**Per-task record.** Success grade; steps against the shortest path; wrong clicks and backtracks; SEQ with a one-line reason; value (1–7, anchored); feeling (3 reaction words plus valence −2…+2, with a reason); and the issues raised.

**Per-issue record.** Title in user language, persona(s) and flow(s), step evidence links, the heuristic violated, kind (bug / UI / content / performance / agent-limit), severity 0–4 from three raters with the mean, frequency (runs seen / runs total), and a proposed fix plus fixing release.

**Ranking.** Sort by:
1. mean severity, rounded up when raters split across the 2/3 boundary;
2. reach (number of personas × core-flow weight);
3. confirmation (seen in ≥ 2/3 runs, or confirmed by the maintainer).

Severity 4, or 3 on a core flow, blocks the announcement. Don't average scores across personas in a way that hides a persona-specific blocker. Report each persona separately first.

**What counts as "high satisfaction".** Published human anchors are SEQ ≈ 5.5 average, SUS 68 average and SUS ≈ 81 for promoters ([MeasuringU](https://measuringu.com/seq10/), [MeasuringU](https://measuringu.com/interpret-sus-score/)). Agent scores are inflated, so "high" should mean clearly above the human "good" band *and* backed by behaviour (§6).

**Presenting it to a decision-maker** (one page first, details on demand):
1. Verdict per persona: ready / ready with fixes / not ready, with the one reason.
2. A scorecard grid of personas × flows: success, median SEQ, behaviour flags, colour-coded against thresholds.
3. The top 10 issues ranked, each with a before/after screenshot pair, severity, reach, fix status and release.
4. What was fixed during the run, and re-test results.
5. Calibration: the agent-vs-maintainer gap on the shared tasks, and recall of the maintainer's known issues.
6. A limits box (what this method cannot tell you).


---

## 6. Recommended protocol for an AI agent

**Roles** (separate contexts, never merged):
- **Builder** (the agent that built AI Juggler): writes personas and flows, sets up fixtures, runs triage and fixes. It never scores.
- **Tester** (fresh agent ×3 per flow): has the information firewall from §3.1 and sees only the brief, the goal and the screen.
- **Critic** (fresh agent): verifies claims against screenshots, classifies failures, challenges unsupported high scores, and rates severity independently.
- **Maintainer**: calibration runs on 2–3 shared tasks; final calls on disputed severities.

**Step 0: Environment.** Use a dedicated test profile with generic fixture data: no real accounts, names or credentials, and no sign-ins. Tools used by the app run read-only or on throwaway accounts. Record app version and commit. Reset to a known state before each run, so first run really is first run.

**Step 1: Personas and flows (Builder, about ½ day).** Use the §4 template with a source on every line. There are 3 personas and 4–6 flows each. Every flow has a goal in user words, a start state, an observable success state, an "abandon if" condition, and the shortest path, which is kept hidden from the tester. Then run the leak list check: the brief contains no internal terms and no UI labels except the ones the persona would plausibly know from other tools.

**Step 2: First impressions (Tester, per persona).**
- **Five-second / clarity test.** Show the first-run screen and the main window once. Ask: "What is this? What is it telling you? What would you do first?" Pass if it names the core purpose and a correct first action.
- **First-click test.** For each flow, show the start screen plus the goal and ask "where do you click first?" before any exploring. Record right or wrong.

**Step 3: The walk (Tester ×3, fresh each time).** Brief: "You are [persona]. You are busy and sceptical; you already manage this without the app. Try to do [goal]. Think aloud. You may give up. Say plainly when something is unclear. Never guess at hidden features; use only what you can see." At every step, fill in the §5 record using predict, act, observe and the four CW questions. Stop rules: give up after 3 attempts that fail to make progress, or after 2× the shortest path plus 5 steps. Loop detection: a repeated screen and action twice in a row means the tester must re-plan or give up. After each task, record SEQ, value, feeling words with valence, and a one-line reason for each.

**Step 4: Confusion pass (Tester, same context, after the walk).** For each screen visited, rate 0/1/2: "would someone like me be confused here, and why?"

**Step 5: End-of-persona questionnaire.** UMUX-Lite (2 items, 1–7), likelihood to recommend (0–10, reported but not gated), the 3 best and 3 worst moments, and "what would make you stop using this?"

**Step 6: Heuristic sweep (separate fresh agent).** Run Nielsen's 10 over the captured screens, with screenshots only. The Builder then checks cross-screen consistency, user control and freedom, and undo by hand, because that is where LLMs are weakest.

**Step 7: Critic pass.** Verify every claim against the screenshot. Strike non-issues and leaked-knowledge steps (the leak lint runs here). Classify failures as UI, agent-limit or environment. Rate severity 0–4 independently. Challenge any SEQ or value of 6–7 that has no behaviour behind it.

**Step 8: Triage and fix (Builder).** Reproduce every bug by hand in the real app. Fix bugs and severity ≥ 3 issues. Re-run the affected flows with a *new* fresh tester; the old one must not re-test its own complaint. Record before/after evidence.

**Step 9: Calibrate.** The maintainer does 2–3 shared tasks (SEQ plus one sentence). Report the gap between agent and maintainer SEQ and the recall of the maintainer's known issues. If agents rate more than 1 SEQ point above the maintainer, raise every threshold by that gap and say so in the report.

**Step 10: Report** in the §5 structure, published as a page with screenshots.

### Scoring rubric (each score gets a one-line reason that cites a step)

| Dimension | Scale | Anchors |
|---|---|---|
| **Success** | S / SD / F / GU | **S** = observable success state, no detours. **SD** = success after a wrong click, backtrack or tie. **F** = wrong end state (including "thought it worked"). **GU** = gave up |
| **Ease (SEQ)** | 1–7 | 7 = did it without thinking, first click right. 6 = one brief hesitation. 5 = noticeable search or one wrong click. 4 = several tries, unsure it worked. 3 = needed luck or trial and error. 2 = barely got there. 1 = could not |
| **Value** | 1–7 | 7 = clearly better than my current way, I'd do it daily. 5 = useful but no better than my alternative. 3 = not worth the effort. 1 = actively worse or harmful (e.g. lost context) |
| **Clarity / confusion** | 0–2 per step | 0 = obvious. 1 = had to look or compare (tie flag, hover). 2 = wrong expectation, or didn't know what happened |
| **Feeling** | valence −2…+2, plus 3 words | Words picked from a balanced list (e.g. confident, in control, relieved, fast, calm / confused, anxious, annoyed, slow, overwhelmed, distrustful) |
| **Severity** | 0–4 (NN/g) | Frequency × impact × persistence; mean of tester, critic and builder ratings |
| **Response time** | ms | ≤ 100 ms instant; ≤ 1 s keeps flow; > 1 s needs a progress cue; > 10 s is a finding regardless ([Nielsen response-time limits](https://www.nngroup.com/articles/response-times-3-important-limits/)) |

### Thresholds

These are deliberately stricter than human norms, because agent scores run high.
- **Flow passes** when all 3 runs reach S or SD, at least 2 of them S, and the first click is right in ≥ 2/3 runs. Median SEQ ≥ 6, median value ≥ 5, no step rated confusion 2 in ≥ 2 runs, and no open issue of severity ≥ 3.
- **Persona "high satisfaction"** requires every core flow to pass, median valence ≥ +1, and UMUX-Lite ≥ 80 (about the SUS promoter band of ~81 and an A−; an agent's "average" of about 68 means the product is *not* good enough). No answer to "what would make you stop" may point at an unfixed issue.
- **Announcement-ready** requires all three personas at high satisfaction, zero open severity-4 issues, zero open severity-3 issues on core flows, and an agent–maintainer calibration gap of ≤ 1 SEQ point. If the gap is larger, it is ready only after the thresholds are adjusted.

**Why this design.** Behavioural measures (success grade, first click, wrong clicks, ties, response time) are what agents report most honestly. Self-ratings stay because they compare with human norms and carry the reasons, but they are gated by behaviour and stricter thresholds because every study found them inflated. Three runs plus a critic follows NN/g's mean-of-three for severity and the measured moderate run-to-run agreement. The firewall and leak lint target self-preference and the curse of knowledge. The maintainer anchor exists because no study shows agent scores match real users.

**What this will not tell you.** Whether strangers with different setups will adopt or pay. How the app feels after weeks of use. Accessibility for real assistive-technology users. Treat the output as a strong pre-launch defect and clarity sweep, not as user-satisfaction data. After the announcement, follow up with 3–5 real outside users per persona.
