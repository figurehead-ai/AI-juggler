# Calm UI research: alarms, attention and colour in monitoring tools

Written 2026-11-14 for the "Christmas tree" complaint about Home and the rail (release/0.5). Sources were checked on that date. Where a source is secondary (a vendor blog or study guide quoting a paywalled standard), it says so.

## 1. What the research agrees on

1. **Colour is a priority signal, so spend it only on priority.** Industrial HMI, alarm management, Apple's HIG, Grafana and calm technology all reach the same rule. The normal state is neutral. Colour, and above all saturated warm colour, belongs to abnormal states that need action.
2. **Repetition destroys the signal.** Clinical alarm fatigue, SRE paging and the fMRI studies of warning habituation all show the same thing: when most signals don't need action, people stop seeing all of them, the real ones included. That is the maintainer's point: "if users feel everything is a disaster then nothing is a disaster".
3. **Budgets work.** Alarm standards don't stop at "use less colour". They set numbers: a priority split of about 80/15/5, a target alarm rate, and a flood threshold. A budget gives us something we can test.
4. **A unique feature pops out, and a shared one doesn't.** Preattentive pop-out only happens when the target has a property its neighbours lack. Amber on 16 elements means no amber element pops out.
5. **Defaults beat modes.** Most users never change settings, so whatever ships as the default is the experience. A "calm mode" would leave the noisy default in front of most users.

## 2. High-performance HMI (ISA-101) and alarm management (ISA-18.2, EEMUA 191)

- **Grey baseline, colour for abnormal states.** High-performance HMI practice under ANSI/ISA-101.01-2015 uses a muted light-grey background (about Munsell N7–N8) and flat 2D graphics, and keeps red and yellow for abnormal states. Normal running equipment is grey, not green. [ISA Interchange: The High Performance HMI](https://blog.isa.org/the-high-performance-hmi); [Tatsoft ISA-101 how-to](https://docs.tatsoft.com/display/FX/ISA-101+HMI+Compliance+How-to); [control.com, Going Gray](https://control.com/technical-articles/going-gray/); [ISA-101 study guide](https://open-exam-prep.com/study-guides/isa-cap/system-design-hmi-alarms-enterprise/hmi-design-isa101) (secondary).
- **Redundant coding.** ISA's own blog calls it "a poor practice" to show an alarm only by recolouring a value or its background. An alarm should be a separate element that is redundantly coded by shape, colour and text, so it shows that the alarm exists *and its priority*. [ISA Interchange](https://blog.isa.org/the-high-performance-hmi).
- **Priority budget.** EEMUA 191 and ISA-18.2 target roughly **80% low, 15% medium and 5% high** priority. [Open Exam Prep, EEMUA 191](https://open-exam-prep.com/study-guides/nebosh-hse-process-safety/process-hazard-control/alarm-management-eemua191) (secondary); [Kim VanCamp, alarm system metrics](https://www.slideshare.net/yousefia20/alarmsystemperformancemetricskimvancamppdf). A thesis on real plants found high priority over-assigned at 5–6× the limit, which is the same failure we have. [Hills, USQ thesis](https://sear.unisq.edu.au/31412/1/Guy_R_Hills.pdf).
- **Rate and flood.** The target in normal operation is under about 1 alarm per 10 minutes. A *flood* is more than 10 alarms in 10 minutes, and it ends when the rate drops below 5. [Honeywell/ASM benchmarking](https://process.honeywell.com/content/dam/process/en/documents/document-lists/doc_asm-consortium/white-papers/February%2028%202005%20-%20Acheiving%20Effective%20Alarm%20System%20Performance%20Benchmarking.pdf); [IChemE alarm rationalisation](https://www.icheme.org/media/17392/lc-0151_21-lead-process-safety-metrics-alarm-rationalisation-final.pdf). ABB stresses that these are guidelines to adapt, not fixed rules. [ABB SCADA alarm white paper](https://library.e.abb.com/public/72f20c70c7b44d889d463db81df5c38d/SCADA%20Alarm%20Management%20White%20Paper.pdf).
- **Shelving.** An operator may suppress a known alarm for a time. AI Juggler's Snooze is the same thing. [processonline, ISA-18.2 part 2](https://www.processonline.com.au/content/software-it/article/improving-alarm-management-with-isa-18-2-part-2-1141142642).

**What it means for us:** "Working" is normal running, so it should be grey or a quiet hue, never cyan and breathing. Priority needs at least three steps (low, medium, high), so the "Asks you" amber must differ from the "crashed" red. A busy moment needs a flood rule that groups the needs instead of painting each one.

## 3. Alert and alarm fatigue

- **Clinical monitoring.** The Joint Commission's Sentinel Event Alert 50 (2013) estimates that **85–99% of alarm signals need no clinical intervention**. It recorded 98 alarm-related events, 80 of them deaths, and alarm safety became a National Patient Safety Goal in 2014. Other studies put false or non-actionable alarms at 72–99%. [Joint Commission SEA 50](https://digitalassets.jointcommission.org/api/public/content/f65e5c9df2b94000a99445e0a7877007); [KFF Health News](https://kffhealthnews.org/news/sounding-an-alarm-on-alarms/).
- **SRE.** Google's SRE book (ch. 6) says: "Every page should be actionable." It adds: "I can only react with a sense of urgency a few times a day before I become fatigued." When pages come too often, people "second-guess, skim, or even ignore incoming alerts". Subcritical problems belong on a dashboard, not in pages. [Google SRE book, Monitoring Distributed Systems](https://sre.google/sre-book/monitoring-distributed-systems/).
- **PagerDuty.** It splits incidents into **high and low urgency**. Only high urgency escalates and interrupts, and low urgency goes to quiet channels. Under dynamic notifications, critical maps to high and warning or info maps to low. [PagerDuty service settings](https://support.pagerduty.com/docs/service-settings); [Dynamic notifications](https://support.pagerduty.com/docs/dynamic-notifications).
- **Habituation is measurable.** BYU's fMRI and eye-tracking studies (Anderson, Vance et al.) show that visual processing of a warning drops sharply after repeated exposure, from "warning to wallpaper". Attention fell over a five-day week, and their recommendation is fewer warnings. [BYU ScholarsArchive 2014](https://scholarsarchive.byu.edu/facpub/9305); [MISQ 2018](https://scholarsarchive.byu.edu/facpub/9293); [BYU News](https://news.byu.edu/news/study-android-users-how-get-people-pay-attention-mobile-security-notifications).

**What it means for us:** each need should be shouted once per surface. Today one "Asks you" appears in amber up to five times on one screen (§audit). Repeating the same amber teaches people to ignore amber.

## 4. Preattentive perception

- A target with a **unique** feature, such as a red item among blue ones, is found in constant time however many distractors there are. A target that shares its features with its neighbours is not. [Healey, Perception in Visualization](https://www.csc2.ncsu.edu/faculty/healey/PP/index.html); [Healey & Enns 2012, IEEE TVCG](https://www.csc2.ncsu.edu/faculty/healey/download/tvcg.12a.pdf).
- **What it means for us:** amber pops out only when amber is rare. Weight and luminance (a dark badge on a light page, or bold text) are a second channel that doesn't use up hue.

## 5. How good monitoring and dev tools handle it

| Tool | What it does | Source |
|---|---|---|
| **Grafana** | Asks for "meaningful use of color" and says "Dashboards should reduce cognitive load, not add to it". Its example is blue for good and red for bad, set by thresholds. | [Grafana dashboard best practices](https://grafana.com/docs/grafana/latest/visualizations/dashboards/build-dashboards/best-practices/) |
| **Datadog** | Monitors have four states (OK, Warn, Alert, No Data). Muting is kept separate from state, so a muted monitor keeps its state but stops notifying. | [Datadog monitor status](https://docs.datadoghq.com/monitors/status/graphs/) |
| **PagerDuty** | Two urgencies with different channels and escalation (above). | above |
| **Linear** (2024, 2026) | 2024: adjusted the sidebar, tabs and headers "to reduce the visual noise". March 2026: the sidebar is "a few notches dimmer", elements that support navigation "should recede", the grey is warmer and "less saturated", there are fewer and smaller icons, coloured team-icon backgrounds are gone, border contrast is softened, and dividers that had "quietly proliferated" were removed. | [How we redesigned the Linear UI](https://linear.app/now/how-we-redesigned-the-linear-ui); [Behind the latest design refresh](https://linear.app/now/behind-the-latest-design-refresh) |
| **GitHub Primer** | Functional colour roles are a ladder: accent (blue), success, **attention** (yellow), **severe** (orange), **danger** (red, "reserved for errors or blocking states… where action is required"). | [Primer colour overview](https://primer.style/foundations/color/overview) |
| **Vercel Geist** | A 10-step grey scale carries the UI. Red means errors, amber warnings and blue information, and every semantic colour meets 4.5:1. | [Geist colours](https://vercel.com/geist/colors); [Geist terminal](https://vercel.com/geist/terminal) |
| **Apple HIG** | Use colour sparingly in non-game apps and consistently for status. Don't rely on colour alone, and use text or glyph shapes. With Liquid Glass, keep colour for status indicators and key actions. | [HIG Color](https://developer.apple.com/design/human-interface-guidelines/color) |
| **Activity Monitor** | Tables are monochrome. Colour appears in the Memory Pressure graph, where green is fine and yellow or red means act. | [Apple: Check if your Mac needs more RAM](https://support.apple.com/guide/activity-monitor/actmntr34865/mac) |
| **Raycast** | Monochrome lists, one accent, and colour only in the item's own icon. It ranks by frecency, so order carries importance instead of colour. | [Raycast manual](https://manual.raycast.com/search-bar) |
| **Claude desktop app** (observed, not documented) | Warm neutral greys, one brand accent kept for the send action and selection, and no status colours in the sidebar. Recency and position carry importance. The maintainer cites it as the calm reference, but it has almost no alerts. | maintainer's reference; observed |

**Pattern across them:** a deep neutral scale does most of the work. One accent goes to the primary action and selection. Status colours are a ranked ladder, each used only when its condition holds. Navigation chrome sits dimmer than content, and identity colour stays small (an icon or a dot, never a border plus a dot plus a chip).

## 6. Calm technology

- Weiser and Brown (Xerox PARC, 1995) say calm technology "engages both the center and the periphery of our attention" and moves between them. They warn that design which pushes everything to the centre overwhelms, as an open-plan office does. [Designing Calm Technology](https://calmtech.com/papers/designing-calm-technology).
- Amber Case's principles: technology should "require the smallest possible amount of attention", "make use of the periphery" and "inform and create calm". [calmtech.com](https://calmtech.com/).
- **What it means for us:** Working, Idle, identity and runway-when-healthy belong in the **periphery**. They stay visible and readable but quiet. Only L2 and L3 move to the centre, and they move back once handled.

## 7. Modes, density settings, or one tuned default?

- **Defaults are the experience.** Spool's informal study of Microsoft Word settings files found under 5% of users had changed any setting, and users assumed the defaults were chosen for a reason. [UIE: Do users change their settings?](https://archive.uie.com/brainsparks/2011/09/14/do-users-change-their-settings/). NN/g: users "rarely — if ever — customize", so "it's therefore imperative to retain a good default design". [NN/g Customization](https://www.nngroup.com/articles/customization-personalization/); [NN/g Power of Defaults](https://www.nngroup.com/articles/the-power-of-defaults/).
- **Density settings do earn their place** when they trade **space**, not priority. Gmail has offered Default, Comfortable and Compact since 2011/2018. [Gmail blog 2011](https://gmail.googleblog.com/2011/11/changing-information-density-in-gmails.html). AI Juggler already has Comfortable, Compact and Minimal for the rail (D-25).
- **Modes that change what is loud** split the attention grammar. The legend, the docs, screenshots and support all have to explain two grammars, and every combination needs validating (D-62 / "Validate every combination"). The gallery already holds 1,974 baselines on release/0.5. A second colour mode would add a theme-sized multiplier to the Home, rail and status captures.
- **OS-level preferences are free and expected on macOS.** `prefers-reduced-motion` is already honoured, and `prefers-contrast: more` (Increase contrast) is a standard hook for heavier borders without our own setting. [HIG Accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility).

**Conclusion:** fix the default. Keep density as it is (it trades space), honour the OS contrast and motion settings, and don't add a "calm mode". The full argument is in proposal.md §e.

## 8. Measuring calm

- **The 5-second test** (Perfetti, mid-2000s): show the screen briefly, then ask what stood out. It measures first impressions and visual hierarchy, not usability. Kuric et al. (2024) found that complex screens need longer exposure, so we also run a 10-second variant. [Smashing: five-second testing](https://smashingmagazine.com/2023/12/five-second-testing-case-study); [Kuric et al. 2024, BIT](https://ideas.repec.org/a/taf/tbitxx/v43y2024i13p3209-3236.html).
- **Colourfulness metric** (Hasler & Süsstrunk, SPIE HVEI 2003, DOI 10.1117/12.477378): one number for an image's colourfulness, from opponent colour channels. It correlated over 90% with observers' ratings and is cheap to run on the gallery captures before and after. [EPFL Infoscience 33994](https://infoscience.epfl.ch/record/33994).
- **WCAG 2.2**: 1.4.3 (4.5:1 for text), 1.4.11 (3:1 for non-text that carries meaning) and 1.4.1 (never colour alone). APCA is a supplement for dark mode, as docs/design/PRINCIPLES.md §7 already requires.
