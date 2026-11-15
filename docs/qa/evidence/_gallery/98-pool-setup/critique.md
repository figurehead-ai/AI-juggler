<!-- captures: 01bd3ba7d420 -->
# 98-pool-setup: the pool setup a newcomer understands (AJ-E3011)

Design critique (D-57, AJ-E2814) against the reference screens: Settings' choice lines, the switch dialog's "What does this mean?", and the calm UI round 2 mocks (levels, section labels, surface tone). Every step of New pool, empty (the first pool: the intro full, nothing chosen, the defaults filled in) and filled (a name, two Apps, an App's next session leading, a Reviewer, a pair blocked, waking, the seal), dark and light, 1280×800. Fixture names only.

## Checklist

| Check | Finding |
|---|---|
| Grid | The step list (240 px) and one content column (max 768 px for a form, the Who's in list with its side panel) on the page's 24 px gutter; the foot spans the window. |
| Hierarchy | Step title (15 px semibold) → its why-line (ink-2) → labels (13 px medium, ink) → read-back sentences (ink) → hints (muted). Section labels are the calm small capitals (`section-label`). |
| Spacing | One scale: 20 px between a step's groups, 8 px inside a group, 2 px between list lines (no dividers). |
| Icons | The roles carry their own icons (Lead's crown, Member's list, Reviewer's tick, Observer's eye); Start from scratch its plus. No icon on a section label. |
| Shared controls | Picker (join as, a member's role and own choice, Add a project), the calm Segmented (where, kind of member, what they may do), ChoiceOption (the lead picker, the seal), the app's disclosure (the intro, "What does this mean?"). |
| Disabled reasons | Lead on an App's role picker is greyed with its reason ("Lead is one session…"); Make pool shows only once Who's in is reached. |
| Plain words | No "Level", no "From ↓ · To →", no "Blank": Look only / Work together / Work together and wake each other; "Lead can message…"; "Start from scratch"; each choice read back as a sentence. |
| Type sizes | 12 px floor (the role cards' holders line, segment words); body 14, meta 13. |
| Both themes | Checked dark and light: the selected segment's edge (0.5's selected-ring) reads in both; the talk table's amber Ask first and red Blocked keep AA. |
| 1280×800 | Every step fits without scrolling except Review filled, which scrolls inside its column (its Change buttons keep it keyboard-reachable). |

## Good

- The order is visible on every step (the side list with ticks) and the foot always says what the rest is filled in with, so "Make pool" from Who's in on is safe.
- Who leads is never empty: sessions open now, each App's next session, or "No lead for now" with what a lead adds.
- Colour only where it means something: the talk table's one exception stands out (amber dot, red ⊘), Allowed is muted text.
- The intro is three lines and a small diagram on the first pool, a one-line fold afterwards.

## Weak

- The Everyone else step reads its join line twice (the labelled picker, then the sentence under it); kept, since the sentence is the read-back the spec asks for, but it is the one repeat on the screens.
- The lead picker's radios sit at the right edge (the shared choice line's layout); a long list of open sessions puts them far from the names on a wide window.
- Who's in's side panel counts "0 full · 0 shell only" for a pool of Apps (their sessions aren't open yet); true, but reads as nothing.

## Fixed

- (The coordinator's check) Start from scratch let an Observer message everyone, against its role card ("Watches and reads. Changes nothing.") and every template; its sends are blocked by default now, its own row calm, and the review says "an Observer only reads".
- (The coordinator's check, follow-up) The foot under Who's in to On their own still said "Everyone may message everyone." while the table blocked an Observer; it reuses the Review's sentence now ("…; an Observer only reads."), so table, foot and Review agree.

- (This branch) The step heading lacked weight against its why-line; semibold now.
- (This branch) Review's long filled column had nothing to focus in it (axe: a scrolling region); each sentence now has its Change, back to that step.
- (This branch) "Everyone in this pool's projects" named by its scope: "Everyone in storefront".
