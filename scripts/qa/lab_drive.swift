// AJ-E1612: the sign-in sitting's guarded driver for AI Juggler Dev.
//
// Drives AI Juggler Dev by its pid only, never by name (the installed AI
// Juggler has the same process name, juggler-app), and refuses any pid
// that isn't an "AI Juggler Dev.app". Never moves the pointer and never
// sends a key or a click to another app: buttons are pressed through
// accessibility, keys are posted to Dev's pid only. It raises nothing
// unless --may-activate says so (below).
//
// THE PERSON'S INPUT, NOT THE DRIVER'S. The HID system's idle time counts
// the driver's own posted keys too (AJ-E1612 dry run 3: every typed key
// read as the person's, PAUSED idle=0.0s and 20 s lost per character), so
// the idle time the rules below read is the person's: the driver keeps
// when it posted each key (the run's input ledger, beside its log, kept
// across its commands), and an input event at one of those moments
// (15 ms before to 60 ms after) is its own. Anything else is the person's:
// any pointer move or click (the driver never posts one), a wheel click
// that isn't in the ledger (`scroll` keeps its own there, as keys are
// kept), and any key between two of the driver's (it waits 0.1 s after
// each key or wheel click so such an event shows). Only a key or a wheel
// click of the person's within those few ms of one of the driver's can
// pass as its own; the frontmost+topmost guard still runs before every
// key and every wheel click.
//
// THE CLICK GUARD. Before every press, keystroke, wheel click and resize, both must hold:
//   - the frontmost app (NSWorkspace) is Dev's pid;
//   - the topmost on-screen window at the point (CGWindowList, active
//     Space, front to back) belongs to Dev's pid.
// If either fails, nothing is sent: the driver prints what is in the way
// and waits (AIJ_DRIVE_WAIT seconds, default 120) for the person at the
// Mac to bring Dev forward; still failing, it stops (exit 3), nothing sent.
// Dev with no window on screen at all (accessibility still reads its page:
// minimised, hidden, or on another Space) is the same: it prints "Dev's
// window is off screen (minimised/hidden/other Space)" and waits.
//
// BRINGING DEV FORWARD (--may-activate, or AIJ_DRIVE_MAY_ACTIVATE=1; off by
// default, so a walk never takes the front without asking; the dry run and
// the sitting turn it on).
// Only when the guard fails because another app is frontmost, or because
// Dev's window is off screen (minimised, hidden, another Space), and only:
//   - when nobody has touched the Mac for 20 s (HID idle time,
//     CGEventSource's HID system state);
//   - while the screen isn't locked, no screen saver runs, and this login
//     session is the one on screen (CGSessionCopyCurrentDictionary; can't
//     tell: no);
//   - once per step (one run of this tool), never in a loop: still failing
//     after it, the guard waits as before.
// "Dev's window" is its MAIN window, the one titled "AI Juggler" or "AI
// Juggler · …" (accessibility), matched to CGWindowList by its frame: any
// other of Dev's windows on screen (dry run 3: a 280x168 one) never stands
// for it, so the main window on another Space reads as off screen.
// Bringing it forward, all by Dev's pid and its own windows, never by name
// (the installed AI Juggler is also juggler-app) and never ⌘⇧M: un-hide Dev
// when hidden (NSRunningApplication), un-minimise its main window when
// minimised (AXMinimized false), then make that window main, raise it and
// make Dev frontmost (AX). macOS doesn't switch Spaces for an app that has
// any window on the Space in use (dry run 3: ACTIVATED did nothing for
// hours), so when the main window is still not on screen it is moved here:
// minimised and restored (a window restored from the Dock opens on the
// Space in use), raised again. Each prints its own line: UNHID /
// UNMINIMISED / ACTIVATED / MOVED <UTC time> step=<command> idle=<s>s.
// Then the guard is checked again: Dev still not frontmost, or its main
// window still off screen, stops the step at once (exit 3, a STOPPED line
// saying what was tried; nothing sent), never a wait that can't end. After it, any new input at the Mac (the idle time falls back
// below the time since the activation) stops the run at once (exit 6):
// nothing more is sent and Dev is left where it is (not lowered, not raised
// again). The full guard still runs before every single event.
//
// OPPORTUNISTIC (--opportunistic, or AIJ_DRIVE_OPPORTUNISTIC=1; off by
// default, and without it all of the above is unchanged). Only the person's
// own keyboard and mouse matter (HID idle time); other agents' apps in
// front are the guard's to wait out.
//   - Pause, don't stop: input at the Mac (HID idle below 20 s) before any
//     event pauses the step: nothing more is sent, Dev is left where it is,
//     and it prints PAUSED <UTC time> step=<command> idle=<s>s. It waits
//     until 20 s pass without input with the screen unlocked, no screen
//     saver, this login on screen; prints RESUMED <UTC time> step=<command>
//     idle=<s>s; then restarts the atomic action it was about to send (one
//     press, one key's down+up, one character: a key is never half sent),
//     the full guard first, and may bring Dev forward once more under the
//     terms above (at most once per resume). A key whose step had put Dev's
//     focus somewhere gets it put back first (REFOCUSED …; can't: exit 6).
//   - No overall time limit while waiting (AIJ_DRIVE_WAIT is ignored), so it
//     can wait overnight; a HEARTBEAT <UTC time> step=<command> … line at
//     most every 10 minutes while it waits or is paused.
//
// Build once (system swiftc only), then run:
//   swiftc -O -o "$S/lab_drive" scripts/qa/lab_drive.swift      # $S: a scratch folder
//   AIJ_DEV_PID=<pid> AIJ_DRIVE_OUT=docs/qa/evidence/AJ-E1612 "$S/lab_drive" <command> …
//   "$S/lab_drive" --selftest                                     # the guard's logic, no clicks
//
// Primitives (TITLE and TEXT are exact words; --row NAME scopes to the one
// row (group) named exactly NAME, and the control must sit inside it; --in
// TEXT scopes to the smallest part of the window holding both TEXT and the
// control, and refuses a part holding two such controls):
//   guard                         the guard's verdict at Dev's window, nothing sent
//   texts [--in TEXT]             the words Dev shows (role, label, value), read only
//   buttons [--in TEXT]           the controls that can be pressed, with their exact titles
//   press TITLE [--row NAME | --region NAME] [--in TEXT …]
//                                 presses the one control titled exactly TITLE; with --row,
//                                 TITLE must name one control in the whole window, inside
//                                 the one row named NAME (each --in adds words its region
//                                 must hold); with --region, the one control of that title
//                                 inside the one part of the window named exactly NAME (a
//                                 landmark such as the header's navigation "Main", a dialog,
//                                 a row): the region's name and the title are the key, so the
//                                 title may name other controls outside it
//   focus TITLE [--in TEXT]       puts the cursor in the field titled TITLE
//   focus-terminal [--in TEXT]    puts the cursor in AI Juggler's terminal (its input)
//   type TEXT                     types TEXT where the cursor is (keys posted to Dev's pid)
//   key NAME [--repeat N]         return, down, up, left, right, pagedown, pageup, home, end,
//                                 escape, tab, space, ctrl-c
//   scroll down|up [N] [--at X,Y] the wheel, N clicks (default 3) of a line, at the screen
//                                 point X,Y (default: the middle of Dev's main window), posted
//                                 to Dev's pid; the pointer doesn't move; each click behind the
//                                 guard at that point
//   resize WxH                    Dev's MAIN window to W×H points, by accessibility, behind
//                                 the guard; prints SIZE; one that didn't take stops (exit 4)
//   windows                       every window of Dev's pid (accessibility's role, subrole,
//                                 title, frame; the window server's layer), read only
//   wait TEXT [--timeout S] [--ocr] [--gone]
//                                 until TEXT shows (or is gone); --ocr also reads the
//                                 window's picture (AI Juggler's terminal draws its text
//                                 on a canvas accessibility can't see)
//   read [--ocr]                  prints the window's words (with --ocr, its picture's)
//   shot NAME (--box X,Y,W,H … | --no-real-data REASON) [--note TEXT]
//                                 Dev's front window by its id, through
//                                 scripts/qa/redact_capture.swift into AIJ_DRIVE_OUT/NAME.jpg
//                                 (+ its sidecar); the raw picture is deleted
//
// The run sheet's steps (docs/qa/0.5-walks/AJ-E1612.md), each a few of the
// above; each prints every primitive it runs, so a step that stops can be
// finished by hand with the same words:
//   sign-in APP                   Home's Needs-you row named "APP needs its sign-in" (APP:
//                                 the App's full name, "OpenCode CLI · lab A"): its button
//                                 "Sign in to APP"; waits for the sign-in terminal ("Sign in · …")
//   read-label APP                APP's sign-in state, only from what Dev derives from it:
//                                 Home's row "APP needs its sign-in" (not-signed-in), or its
//                                 absence on Home with every need shown (signed-in, valid
//                                 after its sign-in terminal ended); the App page's own
//                                 sign-in line verbatim when shown. Prints EVIDENCE: and
//                                 STATE: not-signed-in | signed-in; else stops (exit 4)
//   new-session TOOL APP PROJECT  the App's own "New session: APP" (the rail's row action; the
//                                 header's and the rail's foot's "New session" share one
//                                 name), its sheet's "Change app: APP" checked, Change folder →
//                                 PROJECT only when "Start in PROJECT" isn't offered, then
//                                 "Start in PROJECT" (TOOL names the row in the log)
//   send TEXT [--field TITLE]     types TEXT and Return into the session's terminal
//                                 (or, for a session without one, the field TITLE)
//   end-session ROWTEXT           the session's page (its words ROWTEXT on screen): its one
//                                 "Stop" (SPEC §9.2's stop; held 5 s with Undo, then the
//                                 SIGTERM→SIGKILL ladder), then waits until no Stop or
//                                 "Stopping…" is left and Resume has settled. No "Stop this
//                                 session" exists (AJ-E1612 dry run 3). Stops (exit 4) with
//                                 none or two; by hand then: focus-terminal, key ctrl-c
//   resume ROWTEXT                "Resume" in the row holding ROWTEXT
//   open-usage APP                the header's Usage (--region "Main": a page's own tab is
//                                 "Usage (this session)"), then the words beside APP
//   lab-b-session TOOL APP PROJECT
//                                 new-session on lab B; prints what Dev says instead
//
// THE RUN'S LOG. Every command appends its own lines (the command, each
// primitive, PAUSED/RESUMED/ACTIVATED/MOVED/HEARTBEAT/STOPPED/WAITING,
// EVIDENCE/STATE, its exit) with the UTC time to
// $AIJ_DRIVE_LOG_DIR (default: AIJ_DRIVE_OUT/drive-log)/lab_drive-<run>.log,
// <run> being AIJ_DRIVE_RUN or Dev's launch time and pid; the input ledger
// sits beside it. Never a shared /tmp path (dry run 3's log was deleted
// mid-run by another process). Typed words are kept only by their count,
// and Dev's page content (read, texts, buttons) is printed, never kept.
//
// Exit: 0 done, 2 usage, 3 the guard stopped it (nothing sent), 4 not
// found / ambiguous (nothing sent), 5 timed out waiting, 6 new input at the
// Mac after Dev was brought forward (nothing more sent; opportunistic: Dev's
// focus couldn't be put back after a pause).
import AppKit
import ApplicationServices
import CoreGraphics
import Foundation
import Vision

// MARK: - plain helpers

/// The run's log (`runFile(pid, ".log")`), appended to by every command
/// of the run; nil: none (a selftest, or it couldn't be opened).
var logFile: FileHandle?

/// The driver's own line: printed, and kept in the run's log.
func say(_ text: String) { print(text); fflush(stdout); record(text) }

/// Dev's page content (`read`, `texts`, `buttons`): printed only, never
/// kept in the log (a terminal's picture can show a sign-in code).
func show(_ text: String) { print(text); fflush(stdout) }

/// A line as the log keeps it (pure: the selftest): typed words only by
/// their count (`» type (12 characters)`), stamped with the UTC time.
func logged(_ text: String, at: Double) -> String {
    var line = text
    if line.hasPrefix("» type \"") {
        let inner = line.dropFirst("» type \"".count)
        let typed = inner.hasSuffix("\" + return") ? inner.dropLast("\" + return".count) : inner.hasSuffix("\"") ? inner.dropLast() : inner
        line = "» type (\(characters(typed.count)))" + (inner.hasSuffix(" + return") ? " + return" : "")
    }
    return "\(utc(at)) \(line)"
}

func record(_ text: String) {
    guard let logFile else { return }
    logFile.write((logged(text, at: Date().timeIntervalSince1970) + "\n").data(using: .utf8)!)
}

func characters(_ n: Int) -> String { n == 1 ? "1 character" : "\(n) characters" }

/// The command line as the log keeps it (pure: the selftest): the words
/// of `type` and `send` only by their count.
func runLine(_ args: [String]) -> String {
    guard let command = args.first else { return "RUN (no command)" }
    let rest = Array(args.dropFirst())
    let words = ["type", "send"].contains(command)
        ? "(\(characters(parse(rest).positional.joined(separator: " ").count)))"
        : rest.map { $0.contains(" ") ? "\"\($0)\"" : $0 }.joined(separator: " ")
    return "RUN \(command) \(words)".trimmingCharacters(in: .whitespaces)
}

func stop(_ code: Int32, _ message: String) -> Never {
    FileHandle.standardError.write("lab_drive: \(message)\n".data(using: .utf8)!)
    record("EXIT \(code) \(message)")
    exit(code)
}

func squash(_ text: String) -> String {
    text.split(whereSeparator: { $0 == " " || $0 == "\n" || $0 == "\t" || $0 == "\u{00a0}" }).joined(separator: " ")
}

// MARK: - the guard (pure: tested by --selftest)

struct Win {
    let owner: pid_t
    let ownerName: String
    let title: String
    let layer: Int
    let alpha: Double
    let bounds: CGRect
}

enum Verdict: Equatable {
    case clear
    case notFrontmost(String)
    case covered(String)
    case offWindow
    /// Dev has no window on screen at all (minimised, hidden, another Space).
    case offScreen
}

/// The guard's decision. `windows` is CGWindowList's on-screen list (front
/// to back, the active Space); `point` is in global top-left coordinates.
func judge(dev: pid_t, frontmost: pid_t?, frontmostName: String, windows: [Win], point: CGPoint) -> Verdict {
    guard frontmost == dev else { return .notFrontmost(frontmostName.isEmpty ? "another app" : frontmostName) }
    // Fully transparent windows draw nothing and take no clicks.
    for w in windows where w.alpha > 0.01 && w.bounds.contains(point) {
        if w.owner == dev { return .clear }
        let what = w.title.isEmpty ? w.ownerName : "\(w.ownerName) (\(w.title))"
        return .covered("\(what), layer \(w.layer)")
    }
    return .offWindow
}

func describe(_ verdict: Verdict) -> String {
    switch verdict {
    case .clear: return "clear: Dev is in front and its window is on top at the point"
    case .notFrontmost(let name): return "AI Juggler Dev isn't the frontmost app (\(name) is)"
    case .covered(let what): return "another window is on top at the point: \(what)"
    case .offWindow: return "no Dev window is on screen at the point"
    case .offScreen: return "Dev's window is off screen (minimised/hidden/other Space)"
    }
}

// MARK: - bringing Dev forward (pure: tested by --selftest)

/// Nobody has touched the Mac for this long before Dev may be brought forward.
let minIdle: Double = 20

/// What the guard and the Mac say at one moment.
struct Probe: Equatable {
    var verdict: Verdict
    /// Seconds since the person's last input (HID).
    var idle: Double
    var locked: Bool
    var saver: Bool
    var onConsole: Bool
}

enum Gate: Equatable {
    case send
    case wait(String)
    case activate
    case stopNewInput
    /// Opportunistic: the person is using the Mac; nothing is sent.
    case pause
}

/// While waiting, a heartbeat line at most this often (opportunistic).
let heartbeatEvery: Double = 600

/// What to do now. `activatedAt`: when this step brought Dev forward (nil:
/// not yet, or not since the last resume); `now` and it on one clock, in
/// seconds. `opportunistic`: input pauses instead of stopping.
func decide(_ p: Probe, may: Bool, activatedAt: Double?, now: Double, opportunistic: Bool = false) -> Gate {
    // Opportunistic: any input in the last 20 s pauses, before any event,
    // whatever the guard says.
    if opportunistic && p.idle < minIdle { return .pause }
    // After an activation, the person's input since then ends the run,
    // whatever the guard says (checked before every single event).
    if !opportunistic, let at = activatedAt, now - p.idle > at + 0.25 { return .stopNewInput }
    if p.verdict == .clear { return .send }
    let why = describe(p.verdict)
    guard may else { return .wait(why) }
    guard activatedAt == nil else { return .wait("\(why); Dev was brought forward once this step and never is again") }
    // Only another app in front, or Dev's window off screen: a window over
    // Dev's isn't moved.
    switch p.verdict {
    case .notFrontmost, .offScreen: break
    default: return .wait(why)
    }
    if !p.onConsole { return .wait("\(why); this login isn't the one on screen") }
    if p.locked { return .wait("\(why); the screen is locked") }
    if p.saver { return .wait("\(why); the screen saver runs") }
    if p.idle < minIdle {
        return .wait("\(why); the Mac is in use (last input \(Int(p.idle)) s ago; \(Int(minIdle)) s without input needed)")
    }
    return .activate
}

/// Why a pause still holds (nil: it may resume): 20 s without input, the
/// screen unlocked, no screen saver, this login on screen.
func pauseHolds(_ p: Probe) -> String? {
    if p.idle < minIdle { return "the Mac is in use (last input \(Int(p.idle)) s ago)" }
    if !p.onConsole { return "this login isn't the one on screen" }
    if p.locked { return "the screen is locked" }
    if p.saver { return "the screen saver runs" }
    return nil
}

enum GateEnd: Equatable {
    case send
    /// Opportunistic: paused and resumed; the caller restarts its atomic
    /// action from the start (the guard first), nothing of it was sent.
    case restart
    case guardStopped(String)
    case inputStopped
}

/// One part of bringing Dev forward, by its log line's first word.
enum Raise: String {
    case unhid = "UNHID"
    case unminimised = "UNMINIMISED"
    case activated = "ACTIVATED"
    case moved = "MOVED"
}

/// A part done, and whether the system took it.
struct Raised: Equatable {
    let what: Raise
    let ok: Bool
}

/// The clock and the Mac, injectable (the selftest's are made up).
struct Mac {
    var now: () -> Double
    var sleep: (Double) -> Void
    var probe: () -> Probe
    /// Brings Dev forward: un-hides it and un-minimises its main window
    /// when they need it, then activates it; each part done, in order.
    var activate: () -> [Raised]
    var log: (String) -> Void
}

func utc(_ seconds: Double) -> String {
    let f = ISO8601DateFormatter()
    f.formatOptions = [.withInternetDateTime]
    return f.string(from: Date(timeIntervalSince1970: seconds))
}

/// Opportunistic: the person is using the Mac. Sends nothing and moves
/// nothing; waits until `pauseHolds` lets go, a heartbeat at most every 10
/// minutes.
func pause(step: String, first p: Probe, mac: Mac) {
    let at = mac.now()
    mac.log("PAUSED \(utc(at)) step=\(step) idle=\(String(format: "%.1f", p.idle))s "
        + "(someone is using the Mac: nothing more is sent, Dev stays where it is; it resumes after \(Int(minIdle)) s without input)")
    var beat = at
    while true {
        mac.sleep(1)
        let q = mac.probe()
        let now = mac.now()
        guard let why = pauseHolds(q) else {
            mac.log("RESUMED \(utc(now)) step=\(step) idle=\(String(format: "%.1f", q.idle))s (the guard is checked again before anything is sent)")
            return
        }
        if now - beat >= heartbeatEvery {
            mac.log("HEARTBEAT \(utc(now)) step=\(step) paused \(Int((now - at) / 60)) min: \(why)")
            beat = now
        }
    }
}

/// Waits until an event may be sent: the guard clear (bringing Dev forward
/// at most once, under `decide`'s rules), else stops. `activatedAt` lives
/// for the whole step. Opportunistic: input pauses (`.restart` after it
/// resumes, with a fresh activation allowed), and there is no time limit.
func gate(step: String, may: Bool, opportunistic: Bool = false, activatedAt: inout Double?, mac: Mac, limit: Double) -> GateEnd {
    let start = mac.now()
    var asked = ""
    var beat = start
    while true {
        let p = mac.probe()
        let now = mac.now()
        switch decide(p, may: may, activatedAt: activatedAt, now: now, opportunistic: opportunistic) {
        case .send:
            return .send
        case .pause:
            pause(step: step, first: p, mac: mac)
            // At most one activation per resume.
            activatedAt = nil
            return .restart
        case .stopNewInput:
            mac.log("STOPPED: someone used the Mac after AI Juggler Dev was brought forward (last input \(Int(p.idle)) s ago). "
                + "Nothing more is sent; Dev stays where it is.")
            return .inputStopped
        case .activate:
            activatedAt = now
            // One line each, in the order done (un-hide, un-minimise, activate).
            let parts = mac.activate()
            for part in parts {
                mac.log("\(part.what.rawValue) \(utc(now)) step=\(step) idle=\(String(format: "%.1f", p.idle))s"
                    + (part.ok ? "" : " (the system refused)"))
            }
            // Checked again: Dev frontmost with its main window on screen,
            // or the step stops here (never a wait that can't end).
            mac.sleep(0.4)
            var after = mac.probe()
            for _ in 0..<5 where after.verdict != .clear {
                mac.sleep(0.4)
                after = mac.probe()
            }
            switch after.verdict {
            case .notFrontmost, .offScreen:
                let tried = parts.map { $0.what.rawValue + ($0.ok ? "" : " (refused)") }.joined(separator: ", ")
                let why = "\(describe(after.verdict)) after bringing it forward (\(tried))"
                mac.log("STOPPED \(utc(mac.now())) step=\(step): \(why). Nothing was sent. "
                    + "Bring AI Juggler Dev's main window to this screen yourself (Mission Control, or its Dock icon), then run the step again.")
                return .guardStopped(why)
            default:
                break // clear, or a window over Dev's: the loop decides
            }
        case .wait(let why):
            if why != asked {
                let ask = may
                    ? "Nothing is sent. Dev is brought forward once, after \(Int(minIdle)) s without input; or bring it forward yourself."
                    : "Nothing is sent. Please bring AI Juggler Dev forward (click its window, or its Dock icon); the driver never does it itself."
                mac.log("WAITING: \(why). \(ask) "
                    + (opportunistic ? "Waiting with no time limit (opportunistic), a heartbeat every \(Int(heartbeatEvery / 60)) min." : "Waiting up to \(Int(limit)) s."))
                asked = why
            }
            if opportunistic {
                if now - beat >= heartbeatEvery {
                    mac.log("HEARTBEAT \(utc(now)) step=\(step) waiting \(Int((now - start) / 60)) min: \(why)")
                    beat = now
                }
            } else if now - start > limit {
                return .guardStopped(why)
            }
            mac.sleep(0.5)
        }
    }
}

// MARK: - Dev, by pid only

let env = ProcessInfo.processInfo.environment
let rawArgs = Array(CommandLine.arguments.dropFirst())
/// --may-activate (anywhere on the line), or AIJ_DRIVE_MAY_ACTIVATE=1 (a run
/// sheet's setup, the agent's shell keeping no options between commands):
/// this step may bring Dev forward, under the rules above. Default off.
let mayActivate = rawArgs.contains("--may-activate") || env["AIJ_DRIVE_MAY_ACTIVATE"] == "1"
/// --opportunistic, or AIJ_DRIVE_OPPORTUNISTIC=1: input pauses the step
/// instead of stopping it, and waiting has no time limit (see the head).
let opportunistic = rawArgs.contains("--opportunistic") || env["AIJ_DRIVE_OPPORTUNISTIC"] == "1"
let args = rawArgs.filter { $0 != "--may-activate" && $0 != "--opportunistic" }
let outDir = URL(fileURLWithPath: env["AIJ_DRIVE_OUT"] ?? FileManager.default.currentDirectoryPath)
let waitLimit = Double(env["AIJ_DRIVE_WAIT"] ?? "") ?? 120
/// The run's own folder, for its log and its input ledger: AIJ_DRIVE_LOG_DIR,
/// else drive-log/ in the run's evidence folder (AIJ_DRIVE_OUT); never a
/// shared /tmp path another process's cleanup can sweep (dry run 3 lost
/// its log so).
let runDir = URL(fileURLWithPath: env["AIJ_DRIVE_LOG_DIR"] ?? outDir.appendingPathComponent("drive-log").path)

/// The run's name (pure: the selftest): AIJ_DRIVE_RUN (letters, digits,
/// . _ - only), else Dev's launch time and pid, so every command of one
/// Dev launch shares it and no other run does.
func runName(given: String?, pid: pid_t, launched: Date?) -> String {
    if let given {
        let kept = String(given.unicodeScalars.filter { CharacterSet.alphanumerics.contains($0) && $0.isASCII || "._-".unicodeScalars.contains($0) })
        if !kept.isEmpty { return kept }
    }
    let f = DateFormatter()
    f.locale = Locale(identifier: "en_US_POSIX")
    f.timeZone = TimeZone(identifier: "UTC")
    f.dateFormat = "yyyyMMdd'T'HHmmss'Z'"
    return "dev\(pid)-\(launched.map(f.string) ?? "unknown")"
}

/// The run's file named `suffix`, in its folder.
func runFile(_ pid: pid_t, _ suffix: String) -> URL {
    let name = runName(given: env["AIJ_DRIVE_RUN"], pid: pid, launched: NSRunningApplication(processIdentifier: pid)?.launchDate)
    return runDir.appendingPathComponent("lab_drive-\(name)\(suffix)")
}

func devPid() -> pid_t {
    guard let text = env["AIJ_DEV_PID"], let pid = pid_t(text), pid > 1 else {
        stop(2, "set AIJ_DEV_PID to AI Juggler Dev's pid (lab_scenario.sh open prints it)")
    }
    guard let app = NSRunningApplication(processIdentifier: pid) else { stop(2, "no app runs as pid \(pid)") }
    // Never the installed AI Juggler: only a bundle named "AI Juggler Dev.app".
    guard app.bundleURL?.lastPathComponent == "AI Juggler Dev.app" else {
        stop(2, "pid \(pid) isn't AI Juggler Dev (\(app.bundleURL?.lastPathComponent ?? "no bundle")): refusing")
    }
    return pid
}

func onScreenWindows() -> [Win] {
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
    return list.compactMap { w in
        guard let raw = w[kCGWindowBounds as String] as? NSDictionary, let bounds = CGRect(dictionaryRepresentation: raw) else { return nil }
        return Win(owner: w[kCGWindowOwnerPID as String] as? pid_t ?? -1,
                   ownerName: w[kCGWindowOwnerName as String] as? String ?? "",
                   title: w[kCGWindowName as String] as? String ?? "",
                   layer: w[kCGWindowLayer as String] as? Int ?? 0,
                   alpha: w[kCGWindowAlpha as String] as? Double ?? 1,
                   bounds: bounds)
    }
}

/// One of Dev's windows as CGWindowList has it, on any Space.
struct CGWin: Equatable {
    let id: CGWindowID
    let bounds: CGRect
    let title: String
    /// On the Space in use and not minimised or hidden.
    let onScreen: Bool
}

/// Dev's windows (layer 0), on every Space.
func devCGWindows(_ pid: pid_t) -> [CGWin] {
    let list = CGWindowListCopyWindowInfo([.optionAll, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
    return list.compactMap { w in
        guard (w[kCGWindowOwnerPID as String] as? pid_t) == pid, (w[kCGWindowLayer as String] as? Int) == 0,
              let id = w[kCGWindowNumber as String] as? CGWindowID,
              let raw = w[kCGWindowBounds as String] as? NSDictionary,
              let bounds = CGRect(dictionaryRepresentation: raw) else { return nil }
        return CGWin(id: id, bounds: bounds, title: w[kCGWindowName as String] as? String ?? "",
                     onScreen: (w[kCGWindowIsOnscreen as String] as? Bool) ?? false)
    }
}

/// The main window's title: "AI Juggler", or "AI Juggler · <screen>"
/// (ui/src/lib/window-title.ts).
func isMainTitle(_ title: String) -> Bool {
    let t = squash(title)
    return t == "AI Juggler" || t.hasPrefix("AI Juggler · ")
}

/// Which of Dev's accessibility windows is its main window (pure: the
/// selftest): the one with the main window's title; of two, the one
/// accessibility calls main, else the largest. Never a window of another
/// title, however it sits (dry run 3's 280x168 one).
func pickMain(_ windows: [(title: String, frame: CGRect, isMain: Bool)]) -> Int? {
    let titled = windows.indices.filter { isMainTitle(windows[$0].title) }
    if titled.count <= 1 { return titled.first }
    if let main = titled.first(where: { windows[$0].isMain }) { return main }
    return titled.max { windows[$0].frame.width * windows[$0].frame.height < windows[$1].frame.width * windows[$1].frame.height }
}

/// The CGWindowList window that is the one framed `frame` (within 2 pt),
/// the title deciding between two (pure: the selftest).
func matchWindow(frame: CGRect, title: String, among: [CGWin]) -> CGWin? {
    let close = { (a: CGFloat, b: CGFloat) in abs(a - b) <= 2 }
    let same = among.filter { close($0.bounds.minX, frame.minX) && close($0.bounds.minY, frame.minY)
        && close($0.bounds.width, frame.width) && close($0.bounds.height, frame.height) }
    if same.count <= 1 { return same.first }
    let titled = same.filter { squash($0.title) == squash(title) }
    return titled.count == 1 ? titled.first : nil
}

/// Dev's main window: its accessibility element and its CGWindowList entry.
func mainWindow(_ pid: pid_t) -> (ax: AXUIElement, cg: CGWin)? {
    let app = AXUIElementCreateApplication(pid)
    let windows = (attr(app, kAXWindowsAttribute) as? [AXUIElement]) ?? []
    let main = attr(app, kAXMainWindowAttribute)
    let described = windows.map { w in
        (title: text(w, kAXTitleAttribute), frame: frame(w) ?? .zero,
         isMain: main.map { CFGetTypeID($0) == AXUIElementGetTypeID() && CFEqual($0, w) } ?? false)
    }
    guard let i = pickMain(described), let cg = matchWindow(frame: described[i].frame, title: described[i].title, among: devCGWindows(pid)) else { return nil }
    return (windows[i], cg)
}

/// Dev's main window when it is on screen (the Space in use): its id and
/// bounds; nil when it is minimised, hidden, on another Space, or gone.
func devWindow(_ pid: pid_t) -> (id: CGWindowID, bounds: CGRect, title: String)? {
    guard let main = mainWindow(pid), main.cg.onScreen, main.cg.bounds.width > 50, main.cg.bounds.height > 50 else { return nil }
    return (main.cg.id, main.cg.bounds, main.cg.title)
}

/// Seconds since the last input event of any kind, by the HID system's
/// state (keys, pointer, trackpad), the driver's own posted keys included.
func hidIdle() -> Double {
    CGEventSource.secondsSinceLastEventType(.hidSystemState, eventType: CGEventType(rawValue: ~0)!)
}

/// The pointer's events, and the trackpad's gestures (NSEvent's gesture,
/// magnify, swipe, rotate, smart magnify): never posted by the driver. The
/// wheel isn't here: `scroll` posts it, kept in the ledger as keys are, so
/// the person's wheel shows as the latest input that isn't the driver's.
let pointerEvents: [CGEventType] = [.mouseMoved, .leftMouseDown, .leftMouseUp, .rightMouseDown, .rightMouseUp,
                                    .otherMouseDown, .otherMouseUp, .leftMouseDragged, .rightMouseDragged,
                                    .otherMouseDragged, .tabletPointer, .tabletProximity]
    + [29, 30, 31, 18, 32].compactMap { CGEventType(rawValue: $0) }

/// Seconds since the person's last pointer or trackpad input.
func pointerIdle() -> Double {
    pointerEvents.map { CGEventSource.secondsSinceLastEventType(.hidSystemState, eventType: $0) }.min() ?? 0
}

/// An event this long before or after one of the driver's own posts is
/// its own (the window server stamps it a moment after the post).
let ownBefore = 0.015
let ownAfter = 0.06
/// After each key, a pause before the next, so a key of the person's
/// between two of the driver's shows as the latest input (pure: the selftest).
let keyGap = 0.1

/// The driver's own posts and the person's last input it has seen, kept
/// across its commands (pure: tested by --selftest).
struct InputLedger: Codable, Equatable {
    /// When each of the driver's own key events was posted (seconds since
    /// 1970), the last two minutes' only.
    var own: [Double] = []
    /// The person's last input seen (seconds since 1970); nil: none yet.
    var personAt: Double?

    func isOwn(_ at: Double) -> Bool { own.contains { at >= $0 - ownBefore && at <= $0 + ownAfter } }

    /// Seconds since the person's last input. `any`: since the last input
    /// event of any kind (the HID state, the driver's keys included);
    /// `pointer`: since the last pointer or trackpad event (never the
    /// driver's). The latest event one of the driver's own: the person's
    /// last is the one seen before it, or a pointer event since; none seen
    /// (the ledger lost): `any`, as before (a pause, never a send early).
    mutating func personIdle(any: Double, pointer: Double, now: Double) -> Double {
        let last = now - any
        if isOwn(last) { return min(pointer, personAt.map { now - $0 } ?? any) }
        personAt = max(personAt ?? last, last)
        return any
    }

    mutating func noteOwn(_ at: Double) {
        own.append(at)
        own.removeAll { at - $0 > 120 }
    }
}

/// The run's input ledger, beside its log (`runFile`); nil: a selftest.
var ledgerFile: URL?
var ledger = InputLedger()

func loadLedger() {
    guard let file = ledgerFile, let data = try? Data(contentsOf: file),
          let kept = try? JSONDecoder().decode(InputLedger.self, from: data) else { return }
    ledger = kept
}

func saveLedger() {
    guard let file = ledgerFile, let data = try? JSONEncoder().encode(ledger) else { return }
    try? FileManager.default.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
    try? data.write(to: file, options: .atomic)
}

/// Seconds since the person's last input (not the driver's own keys).
func personIdle() -> Double {
    let before = ledger
    let idle = ledger.personIdle(any: hidIdle(), pointer: pointerIdle(), now: Date().timeIntervalSince1970)
    if ledger != before { saveLedger() }
    return idle
}

/// The login session's lock and console state; can't tell: locked, off console.
func session() -> (locked: Bool, onConsole: Bool) {
    guard let info = CGSessionCopyCurrentDictionary() as? [String: Any] else { return (true, false) }
    let locked = (info["CGSSessionScreenIsLocked"] as? Bool) ?? ((info["CGSSessionScreenIsLocked"] as? Int).map { $0 != 0 } ?? false)
    let onConsole = (info[kCGSessionOnConsoleKey as String] as? Bool) ?? false
    return (locked, onConsole)
}

func screenSaverRuns() -> Bool {
    NSWorkspace.shared.runningApplications.contains {
        $0.bundleIdentifier == "com.apple.ScreenSaver.Engine" || $0.localizedName == "ScreenSaverEngine"
    }
}

/// Dev's main window when it is minimised (else its first minimised
/// standard window); nil when none is.
func minimisedWindow(_ app: AXUIElement) -> AXUIElement? {
    let minimised = { (w: AXUIElement) in (attr(w, kAXMinimizedAttribute) as? Bool) == true }
    if let main = attr(app, kAXMainWindowAttribute), CFGetTypeID(main) == AXUIElementGetTypeID() {
        let window = main as! AXUIElement
        if minimised(window) { return window }
    }
    let windows = (attr(app, kAXWindowsAttribute) as? [AXUIElement]) ?? []
    return windows.first { minimised($0) && text($0, kAXSubroleAttribute) == (kAXStandardWindowSubrole as String) }
        ?? windows.first(where: minimised)
}

/// What bringing Dev forward works on: Dev's own app and main window, by
/// its pid (the selftest's are made up).
struct Forward {
    var hidden: () -> Bool
    var unhide: () -> Bool
    var minimised: () -> Bool
    var unminimise: () -> Bool
    /// The main window made main and raised, Dev made frontmost.
    var activate: () -> Bool
    /// The main window is on screen (the Space in use).
    var onThisSpace: () -> Bool
    /// The main window minimised and restored: back on the Space in use.
    var moveHere: () -> Bool
    var settle: (Double) -> Void
}

/// Brings Dev forward, each part done in order (pure: tested by
/// --selftest): un-hide, un-minimise, activate; still not on this Space,
/// moved here and activated again.
func bringForwardSteps(_ f: Forward) -> [Raised] {
    var done: [Raised] = []
    if f.hidden() { done.append(Raised(what: .unhid, ok: f.unhide())) }
    if f.minimised() { done.append(Raised(what: .unminimised, ok: f.unminimise())) }
    done.append(Raised(what: .activated, ok: f.activate()))
    f.settle(0.6)
    guard !f.onThisSpace() else { return done }
    // Activating an app with any window on this Space doesn't switch
    // Spaces (dry run 3): the main window is brought here instead.
    let moved = f.moveHere()
    f.settle(0.4)
    if moved { _ = f.activate() }
    done.append(Raised(what: .moved, ok: moved && f.onThisSpace()))
    return done
}

/// Polls `until` up to `seconds`; whether it held.
func within(_ seconds: Double, _ until: () -> Bool) -> Bool {
    let end = Date(timeIntervalSinceNow: seconds)
    while Date() < end {
        if until() { return true }
        usleep(100_000)
    }
    return until()
}

/// Brings Dev (by pid, its own main window) forward; nothing is clicked or typed.
func bringForward(_ pid: pid_t) -> [Raised] {
    let running = NSRunningApplication(processIdentifier: pid)
    let app = AXUIElementCreateApplication(pid)
    let isMin = { (w: AXUIElement) in (attr(w, kAXMinimizedAttribute) as? Bool) == true }
    return bringForwardSteps(Forward(
        hidden: { running?.isHidden ?? false },
        unhide: { running?.unhide() ?? false },
        minimised: { minimisedWindow(app) != nil },
        unminimise: {
            guard let window = minimisedWindow(app) else { return true }
            return AXUIElementSetAttributeValue(window, kAXMinimizedAttribute as CFString, kCFBooleanFalse) == .success
        },
        activate: {
            if let main = mainWindow(pid)?.ax {
                AXUIElementSetAttributeValue(main, kAXMainAttribute as CFString, kCFBooleanTrue)
                AXUIElementPerformAction(main, kAXRaiseAction as CFString)
            }
            return AXUIElementSetAttributeValue(app, kAXFrontmostAttribute as CFString, kCFBooleanTrue) == .success
                || (running?.activate(options: []) ?? false)
        },
        onThisSpace: { within(1.5) { devWindow(pid) != nil } },
        moveHere: {
            guard let main = mainWindow(pid)?.ax,
                  AXUIElementSetAttributeValue(main, kAXMinimizedAttribute as CFString, kCFBooleanTrue) == .success,
                  within(3, { isMin(main) }) else { return false }
            AXUIElementSetAttributeValue(main, kAXMinimizedAttribute as CFString, kCFBooleanFalse)
            guard within(3, { !isMin(main) }) else { return false }
            AXUIElementPerformAction(main, kAXRaiseAction as CFString)
            return true
        },
        settle: nap))
}

/// The step (command) this run is, for the activation log.
var step = "?"
/// When this step brought Dev forward (once at most).
var activatedAt: Double?

/// Sleeps, letting the run loop take AppKit's notices meanwhile, so
/// NSWorkspace's frontmost app and running apps stay current over a long
/// wait (a tool that never runs its run loop can read them stale).
func nap(_ seconds: Double) {
    let until = Date(timeIntervalSinceNow: seconds)
    _ = RunLoop.current.run(mode: .default, before: until)
    let left = until.timeIntervalSinceNow
    if left > 0 { usleep(useconds_t(left * 1_000_000)) }
}

/// What the guard says to the action about to be sent.
enum Go { case send, restart }

/// The guard, right before a press or a key (`point`: where it lands; nil:
/// the middle of Dev's window, only waiting for one on screen). Waits
/// (asking once) while it fails; stops with nothing sent if it still fails
/// after AIJ_DRIVE_WAIT. With --may-activate it may bring Dev forward once
/// (see the head). With --opportunistic, `.restart` after a pause: the
/// caller starts its atomic action again (nothing of it was sent).
func guarded(_ pid: pid_t, at point: CGPoint?) -> Go {
    let mac = Mac(
        now: { Date().timeIntervalSince1970 },
        sleep: nap,
        probe: {
            let front = NSWorkspace.shared.frontmostApplication
            // No window on screen while accessibility reads the page:
            // minimised, hidden, or on another Space.
            let win = devWindow(pid)
            let verdict = win.map { win in
                judge(dev: pid, frontmost: front?.processIdentifier, frontmostName: front?.localizedName ?? "",
                      windows: onScreenWindows(), point: point ?? CGPoint(x: win.bounds.midX, y: win.bounds.midY))
            } ?? .offScreen
            let s = session()
            return Probe(verdict: verdict, idle: personIdle(), locked: s.locked, saver: screenSaverRuns(), onConsole: s.onConsole)
        },
        activate: { bringForward(pid) },
        log: say)
    switch gate(step: step, may: mayActivate, opportunistic: opportunistic, activatedAt: &activatedAt, mac: mac, limit: waitLimit) {
    case .send: return .send
    case .restart: return .restart
    case .guardStopped(let why): stop(3, "stopped by the guard: \(why). Nothing was sent.")
    case .inputStopped: stop(6, "stopped: new input at the Mac after Dev was brought forward. Nothing more was sent.")
    }
}

// MARK: - accessibility

func attr(_ e: AXUIElement, _ name: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(e, name as CFString, &value) == .success ? value : nil
}

func text(_ e: AXUIElement, _ name: String) -> String { squash((attr(e, name) as? String) ?? "") }

func frame(_ e: AXUIElement) -> CGRect? {
    guard let pv = attr(e, kAXPositionAttribute), let sv = attr(e, kAXSizeAttribute) else { return nil }
    var p = CGPoint.zero, z = CGSize.zero
    guard AXValueGetValue(pv as! AXValue, .cgPoint, &p), AXValueGetValue(sv as! AXValue, .cgSize, &z) else { return nil }
    return CGRect(origin: p, size: z)
}

func actions(_ e: AXUIElement) -> [String] {
    var names: CFArray?
    AXUIElementCopyActionNames(e, &names)
    return (names as? [String]) ?? []
}

struct Node {
    let element: AXUIElement
    let role: String
    let label: String   // its title, else its description
    let value: String
    let parent: Int?
    let depth: Int
    /// Its own words: the label, else (text) the value.
    var words: String { label.isEmpty ? value : label }
}

/// Dev's windows' accessibility trees, flattened (depth first).
func tree(_ pid: pid_t) -> [Node] {
    guard AXIsProcessTrusted() else { stop(3, "this terminal has no accessibility access (System Settings › Privacy › Accessibility)") }
    let app = AXUIElementCreateApplication(pid)
    var nodes: [Node] = []
    func walk(_ e: AXUIElement, _ parent: Int?, _ depth: Int) {
        if depth > 80 || nodes.count > 60_000 { return }
        let label = [text(e, kAXTitleAttribute), text(e, kAXDescriptionAttribute)].first { !$0.isEmpty } ?? ""
        let value = (attr(e, kAXValueAttribute) as? String).map(squash) ?? ""
        nodes.append(Node(element: e, role: text(e, kAXRoleAttribute), label: label, value: value, parent: parent, depth: depth))
        let me = nodes.count - 1
        // R9 (the 0.5.0 re-walk): WebKit's AXTable lists every cell under
        // its AXRow and again under its AXColumn (VoiceOver reads rows), so
        // a column's cells are walked once, through their rows; a press on
        // a row's button is no longer refused as ambiguous.
        if nodes[me].role == "AXColumn" { return }
        for child in (attr(e, kAXChildrenAttribute) as? [AXUIElement]) ?? [] { walk(child, me, depth + 1) }
    }
    for window in (attr(app, kAXWindowsAttribute) as? [AXUIElement]) ?? [] { walk(window, nil, 0) }
    return nodes
}

/// The nodes under `i` (to `levels` levels down), from the flat list's
/// depth-first order: they follow it until the depth returns to its own.
func descendants(_ nodes: [Node], _ i: Int, levels: Int = Int.max) -> [Int] {
    var out: [Int] = []
    var j = i + 1
    while j < nodes.count && nodes[j].depth > nodes[i].depth {
        if nodes[j].depth - nodes[i].depth <= levels { out.append(j) }
        j += 1
    }
    return out
}

func ancestors(_ nodes: [Node], _ i: Int) -> [Int] {
    var out = [i]
    var at = nodes[i].parent
    while let p = at { out.append(p); at = nodes[p].parent }
    return out
}

/// Roles a row (a named group) has: a list item and role=group are AXGroup in WebKit.
let rowRoles: Set<String> = ["AXGroup", "AXRow", "AXCell", "AXOutlineRow"]

/// The one control titled `title` in the whole window, inside the one row
/// named exactly `row`; else nil and why (AJ-E1612: no title may stand
/// for two controls, and a button beside its row isn't its row's).
func pickInRow(_ nodes: [Node], _ candidates: [Int], title: String, row: String) -> (Int?, String) {
    let name = squash(row)
    let rows = nodes.indices.filter { rowRoles.contains(nodes[$0].role) && nodes[$0].label == name }
    guard rows.count == 1, let r = rows.first else {
        return (nil, rows.isEmpty ? "no row named exactly \"\(name)\"" : "\(rows.count) rows named \"\(name)\"")
    }
    guard candidates.count == 1, let c = candidates.first else {
        return (nil, candidates.isEmpty ? "none titled exactly \"\(title)\"" : "\(candidates.count) controls titled exactly \"\(title)\": a title must name one")
    }
    guard ancestors(nodes, c).contains(r) else { return (nil, "the control \"\(title)\" isn't inside the row \"\(name)\"") }
    return (c, "")
}

/// Roles that are only ever a control or a word, never a part of the
/// window that holds controls.
let leafRoles: Set<String> = ["AXButton", "AXMenuItem", "AXMenuButton", "AXPopUpButton", "AXRadioButton", "AXCheckBox",
                              "AXLink", "AXTab", "AXDisclosureTriangle", "AXStaticText", "AXImage", "AXHeading",
                              "AXTextField", "AXTextArea", "AXComboBox", "AXSearchField"]

/// The one control titled `title` inside the one part of the window named
/// exactly `region` (AJ-E1612 dry run 3: the header's "Usage" and a
/// session page's own; a landmark's name and a title make a unique key).
/// Else nil and why: no such part, two parts of that name, none or two of
/// the title inside it.
func pickInRegion(_ nodes: [Node], _ candidates: [Int], title: String, region: String) -> (Int?, String) {
    let name = squash(region)
    let parts = nodes.indices.filter { !leafRoles.contains(nodes[$0].role) && nodes[$0].label == name }
    guard parts.count == 1, let r = parts.first else {
        return (nil, parts.isEmpty ? "no part of the window named exactly \"\(name)\"" : "\(parts.count) parts of the window named \"\(name)\"")
    }
    let inside = candidates.filter { ancestors(nodes, $0).contains(r) }
    guard inside.count == 1, let c = inside.first else {
        return (nil, inside.isEmpty ? "none titled exactly \"\(title)\" inside \"\(name)\"" : "\(inside.count) controls titled exactly \"\(title)\" inside \"\(name)\"")
    }
    return (c, "")
}

let pressRoles: Set<String> = ["AXButton", "AXMenuItem", "AXMenuButton", "AXPopUpButton", "AXRadioButton", "AXCheckBox",
                               "AXLink", "AXTab", "AXCell", "AXRow", "AXDisclosureTriangle"]
let fieldRoles: Set<String> = ["AXTextField", "AXTextArea", "AXComboBox", "AXSearchField"]

/// The controls titled exactly `title`: a pressable element whose own label
/// is the title, or whose one text inside (three levels at most) is.
func controls(_ nodes: [Node], titled title: String, roles: Set<String>) -> [Int] {
    let want = squash(title)
    var found: [Int] = []
    for (i, n) in nodes.enumerated() where roles.contains(n.role) {
        if n.label == want { found.append(i); continue }
        // A web button's title is often its text child's value.
        if n.label.isEmpty {
            let inside = descendants(nodes, i, levels: 3).filter { !nodes[$0].words.isEmpty }
            if inside.count >= 1 && inside.allSatisfy({ nodes[$0].role == "AXStaticText" || nodes[$0].role == "AXGroup" || nodes[$0].role == "AXImage" }) {
                let joined = squash(inside.map { nodes[$0].words }.joined(separator: " "))
                if inside.contains(where: { nodes[$0].words == want }) && (joined == want || inside.filter({ !nodes[$0].words.isEmpty && nodes[$0].role == "AXStaticText" }).count == 1) {
                    found.append(i)
                }
            }
        }
    }
    return found
}

/// The one control of `candidates` (every control with the title in the
/// window), in a region holding every one of `regions`' words. A title
/// that names two controls is refused, words or not (AJ-E1612: the dry
/// run's `--in` tied Kimi Code's Sign in to OpenCode's words, the rows'
/// boxes nested oddly): name a row with --row instead. None: nil, with
/// the reason.
func pick(_ nodes: [Node], _ candidates: [Int], within regions: [String]) -> (Int?, String) {
    guard candidates.count <= 1 else {
        return (nil, "\(candidates.count) controls have that title: a title must name one (press by --row NAME, the row's own name)")
    }
    guard !regions.isEmpty else {
        if candidates.count == 1 { return (candidates[0], "") }
        return (nil, "none found")
    }
    var markUps: [[Set<Int>]] = []  // per region text: each mark's ancestors
    for region in regions {
        let marks = nodes.indices.filter { nodes[$0].words.contains(region) }
        if marks.isEmpty { return (nil, "no words \"\(region)\" on screen") }
        markUps.append(marks.map { Set(ancestors(nodes, $0)) })
    }
    var best: [(Int, Int, Int)] = []  // (candidate, its region, the region's depth)
    for c in candidates {
        // The first ancestor (from the control up) holding every text.
        if let region = ancestors(nodes, c).first(where: { a in markUps.allSatisfy { ups in ups.contains { $0.contains(a) } } }) {
            best.append((c, region, nodes[region].depth))
        }
    }
    let named = regions.map { "\"\($0)\"" }.joined(separator: " + ")
    guard let top = best.map({ $0.2 }).max() else { return (nil, "none found in a region with \(named)") }
    let winners = best.filter { $0.2 == top }
    guard winners.count == 1, let (winner, _, _) = winners.first else { return (nil, "none found in a region with \(named)") }
    return (winner, "")
}

func center(_ e: AXUIElement) -> CGPoint? {
    guard let f = frame(e), f.width > 0, f.height > 0 else { return nil }
    return CGPoint(x: f.midX, y: f.midY)
}

/// Dev's front window on screen. While it is off screen (minimised,
/// hidden, another Space) the guard says so and waits; with --may-activate
/// it brings it back once (see the head); still off screen, it stops.
func windowOnScreen(_ pid: pid_t) -> (id: CGWindowID, bounds: CGRect, title: String) {
    while true {
        if let win = devWindow(pid) { return win }
        if guarded(pid, at: nil) == .restart { continue }
        guard let win = devWindow(pid) else { stop(3, "\(describe(.offScreen)); nothing sent") }
        return win
    }
}

/// A point inside Dev's front window for the element (scrolled into view first).
func pointFor(_ pid: pid_t, _ e: AXUIElement) -> CGPoint {
    let win = windowOnScreen(pid)
    if let c = center(e), win.bounds.contains(c) { return c }
    if actions(e).contains("AXScrollToVisible") {
        AXUIElementPerformAction(e, "AXScrollToVisible" as CFString)
        usleep(250_000)
        if let c = center(e), win.bounds.contains(c) { return c }
    }
    stop(4, "the control isn't inside Dev's window on screen (scroll to it first); nothing sent")
}

// MARK: - keys, posted to Dev's pid only

let keyCodes: [String: CGKeyCode] = ["return": 36, "enter": 36, "tab": 48, "space": 49, "escape": 53, "esc": 53,
                                     "left": 123, "right": 124, "down": 125, "up": 126, "delete": 51, "c": 8,
                                     // The 0.5.0 final walk (F11): scrolling by keyboard.
                                     "pagedown": 121, "pageup": 116, "home": 115, "end": 119]

/// The keys `key` takes, for its usage line.
let keyNames = "return, down, up, left, right, pagedown, pageup, home, end, escape, tab, space, ctrl-c"

func keyPoint(_ pid: pid_t) -> CGPoint {
    let win = windowOnScreen(pid)
    let app = AXUIElementCreateApplication(pid)
    if let focused = attr(app, kAXFocusedUIElementAttribute), let c = center(focused as! AXUIElement),
       win.bounds.contains(c) { return c }
    return CGPoint(x: win.bounds.midX, y: win.bounds.midY)
}

/// Dev's focused element (where its keys land), if any.
func focusedElement(_ pid: pid_t) -> AXUIElement? {
    guard let f = attr(AXUIElementCreateApplication(pid), kAXFocusedUIElementAttribute),
          CFGetTypeID(f) == AXUIElementGetTypeID() else { return nil }
    return (f as! AXUIElement)
}

/// After a pause: Dev's focus back where this step had it before the
/// pause (the person may have clicked in Dev meanwhile), by accessibility,
/// no click; can't: stops, nothing more sent.
func restoreFocus(_ pid: pid_t, _ before: AXUIElement?) {
    guard let before else { return }
    if let now = focusedElement(pid), CFEqual(now, before) { return }
    guard AXUIElementSetAttributeValue(before, kAXFocusedAttribute as CFString, kCFBooleanTrue) == .success else {
        stop(6, "Dev's focus moved during the pause and couldn't be put back, so the step's keys would land elsewhere. Nothing more was sent.")
    }
    usleep(200_000)
    say("REFOCUSED \(utc(Date().timeIntervalSince1970)) step=\(step) (Dev's focus moved during the pause; put back where this step had it)")
}

/// One key, down and up, as one atomic action: the guard before it, never
/// between its down and its up. After a pause it starts again (focus put
/// back first, then the full guard).
func post(_ pid: pid_t, code: CGKeyCode, flags: CGEventFlags = [], unicode: String? = nil) {
    while true {
        let before = focusedElement(pid)
        guard guarded(pid, at: keyPoint(pid)) == .restart else { break }
        restoreFocus(pid, before)
    }
    // A private source; the HID state still counts these keys, so each
    // one's moment goes in the run's ledger (the person's idle time
    // leaves them out).
    let source = CGEventSource(stateID: .privateState)
    for down in [true, false] {
        guard let event = CGEvent(keyboardEventSource: source, virtualKey: code, keyDown: down) else { stop(3, "no key event") }
        event.flags = flags
        if let unicode {
            let units = Array(unicode.utf16)
            event.keyboardSetUnicodeString(stringLength: units.count, unicodeString: units)
        }
        ledger.noteOwn(Date().timeIntervalSince1970)
        event.postToPid(pid)
        usleep(12_000)
    }
    saveLedger()
    // A key of the person's after this one shows as the latest input.
    usleep(useconds_t(keyGap * 1_000_000))
}

func typeText(_ pid: pid_t, _ s: String) {
    for ch in s { post(pid, code: 0, unicode: String(ch)) }
}

func pressKey(_ pid: pid_t, _ name: String) {
    if name == "ctrl-c" { post(pid, code: 8, flags: .maskControl); return }
    guard let code = keyCodes[name] else { stop(2, "unknown key \(name) (\(keyNames))") }
    post(pid, code: code)
}

// MARK: - the wheel and the window's size (the 0.5.0 final walk, F11)

/// "X,Y" as a point (pure: the selftest); nil when it isn't two numbers.
func parsePoint(_ raw: String) -> CGPoint? {
    let parts = raw.split(separator: ",").map { Double($0.trimmingCharacters(in: .whitespaces)) }
    guard parts.count == 2, let x = parts[0], let y = parts[1] else { return nil }
    return CGPoint(x: x, y: y)
}

/// "WxH" (or "W×H") as a size (pure: the selftest); nil unless both are
/// whole numbers from 200 to 10 000.
func parseSize(_ raw: String) -> CGSize? {
    let parts = raw.replacingOccurrences(of: "×", with: "x").lowercased().split(separator: "x").map { Int($0) }
    guard parts.count == 2, let w = parts[0], let h = parts[1], (200...10_000).contains(w), (200...10_000).contains(h) else { return nil }
    return CGSize(width: w, height: h)
}

/// The wheel's lines for `scroll DIRECTION [N]` (pure: the selftest): the
/// sign the wheel's first axis takes (up is positive), N clicks of one
/// line each, 1 to 50; nil for another direction or count.
func wheelLines(_ direction: String, _ count: String?) -> Int32? {
    guard let n = Int32(count ?? "3"), (1...50).contains(n) else { return nil }
    switch direction {
    case "down": return -n
    case "up": return n
    default: return nil
    }
}

/// The wheel at `point` (screen points; nil: the middle of Dev's main
/// window), `lines` clicks one line at a time, posted to Dev's pid only:
/// each click behind the full guard at that point, as a key is. The
/// pointer doesn't move.
func scrollWheel(_ pid: pid_t, lines: Int32, at given: CGPoint?) {
    let step: Int32 = lines < 0 ? -1 : 1
    for _ in 0..<abs(lines) {
        var point = CGPoint.zero
        while true {
            let win = windowOnScreen(pid)
            point = given ?? CGPoint(x: win.bounds.midX, y: win.bounds.midY)
            guard win.bounds.contains(point) else { stop(4, "the point \(Int(point.x)),\(Int(point.y)) isn't inside Dev's main window; nothing sent") }
            if guarded(pid, at: point) == .send { break }
        }
        let source = CGEventSource(stateID: .privateState)
        guard let event = CGEvent(scrollWheelEvent2Source: source, units: .line, wheelCount: 1, wheel1: step, wheel2: 0, wheel3: 0) else { stop(3, "no wheel event") }
        event.location = point
        ledger.noteOwn(Date().timeIntervalSince1970)
        event.postToPid(pid)
        saveLedger()
        usleep(useconds_t(keyGap * 1_000_000))
    }
}

/// Dev's MAIN window to `size` (points), by accessibility and its pid,
/// behind the full guard at the window's middle; never another of its
/// windows (W-03: a size set on the first window accessibility listed
/// moved the small 280x168 one, and the main window kept its size). Says
/// the size it has after; one that didn't take stops (exit 4).
func resizeMain(_ pid: pid_t, to size: CGSize) {
    while true {
        let win = windowOnScreen(pid)
        if guarded(pid, at: CGPoint(x: win.bounds.midX, y: win.bounds.midY)) == .send { break }
    }
    guard let main = mainWindow(pid) else { stop(3, "Dev's main window isn't there; nothing sent") }
    var wanted = size
    guard let value = AXValueCreate(.cgSize, &wanted) else { stop(3, "no size value") }
    let result = AXUIElementSetAttributeValue(main.ax, kAXSizeAttribute as CFString, value)
    usleep(400_000)
    let now = frame(main.ax)?.size ?? .zero
    say("SIZE \(Int(now.width))×\(Int(now.height)) (asked \(Int(size.width))×\(Int(size.height)), accessibility said \(result.rawValue))")
    if abs(now.width - size.width) > 2 || abs(now.height - size.height) > 2 {
        stop(4, "the main window didn't take that size (its minimum is 1024×680, and a full-screen or tiled window keeps its own)")
    }
}

/// Every window of Dev's pid, read only (W-04: which one is the untitled
/// 280x168 one): accessibility's role, subrole, title and frame, and the
/// window server's layer, name and on-screen state for each of its
/// windows, every layer.
func listWindows(_ pid: pid_t) {
    let app = AXUIElementCreateApplication(pid)
    for w in (attr(app, kAXWindowsAttribute) as? [AXUIElement]) ?? [] {
        let f = frame(w) ?? .zero
        show("AX \(text(w, kAXRoleAttribute)) \(text(w, kAXSubroleAttribute)) \"\(text(w, kAXTitleAttribute))\" \(Int(f.width))×\(Int(f.height)) at \(Int(f.minX)),\(Int(f.minY))"
            + " modal=\((attr(w, kAXModalAttribute) as? Bool) ?? false) children=\(((attr(w, kAXChildrenAttribute) as? [AXUIElement]) ?? []).count)")
    }
    let list = CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as? [[String: Any]] ?? []
    for w in list where (w[kCGWindowOwnerPID as String] as? pid_t) == pid {
        let b = (w[kCGWindowBounds as String] as? NSDictionary).flatMap { CGRect(dictionaryRepresentation: $0) } ?? .zero
        show("CG #\(w[kCGWindowNumber as String] as? Int ?? 0) layer=\(w[kCGWindowLayer as String] as? Int ?? 0) \"\(w[kCGWindowName as String] as? String ?? "")\" \(Int(b.width))×\(Int(b.height)) at \(Int(b.minX)),\(Int(b.minY))"
            + " onscreen=\((w[kCGWindowIsOnscreen as String] as? Bool) ?? false) alpha=\(w[kCGWindowAlpha as String] as? Double ?? 1)")
    }
}

// MARK: - reading the picture (OCR, for the terminal's canvas)

func capture(_ pid: pid_t, to file: URL) -> Bool {
    guard let win = devWindow(pid) else { return false }
    try? FileManager.default.removeItem(at: file)
    let task = Process()
    task.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
    task.arguments = ["-x", "-o", "-l\(win.id)", file.path]
    try? task.run()
    task.waitUntilExit()
    return FileManager.default.fileExists(atPath: file.path)
}

/// The words in Dev's front window's picture, line by line. The picture is
/// deleted at once (it may show a sign-in code).
func ocrLines(_ pid: pid_t) -> [String] {
    let file = FileManager.default.temporaryDirectory.appendingPathComponent("lab_drive-ocr-\(getpid()).png")
    defer { try? FileManager.default.removeItem(at: file) }
    guard capture(pid, to: file), let image = NSImage(contentsOf: file),
          let cg = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else { return [] }
    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.usesLanguageCorrection = false
    try? VNImageRequestHandler(cgImage: cg).perform([request])
    let found = (request.results ?? []).compactMap { $0.topCandidates(1).first?.string }
    return found.map(squash)
}

func screenWords(_ pid: pid_t, ocr: Bool) -> [String] {
    var words = tree(pid).map { $0.words }.filter { !$0.isEmpty }
    if ocr { words += ocrLines(pid) }
    return words
}

func shows(_ words: [String], _ needle: String) -> Bool {
    let want = squash(needle).lowercased()
    return words.contains { $0.lowercased().contains(want) } || squash(words.joined(separator: " ")).lowercased().contains(want)
}

// MARK: - the primitives

struct Opts {
    var positional: [String] = []
    var within: [String] = []
    var row: String?
    var region: String?
    var timeout: Double = 30
    var ocr = false
    var gone = false
    var repeatCount = 1
    var field: String?
    var boxes: [String] = []
    var noRealData: String?
    var note: String?
    /// `scroll`'s point, "X,Y" in screen points.
    var at: String?
}

func parse(_ raw: [String]) -> Opts {
    var o = Opts()
    var i = 0
    func value() -> String {
        i += 1
        guard i < raw.count else { stop(2, "\(raw[i - 1]) needs a value") }
        return raw[i]
    }
    while i < raw.count {
        switch raw[i] {
        case "--in": o.within.append(value())
        case "--row": o.row = value()
        case "--region": o.region = value()
        case "--timeout": o.timeout = Double(value()) ?? 30
        case "--ocr": o.ocr = true
        case "--gone": o.gone = true
        case "--repeat": o.repeatCount = max(1, Int(value()) ?? 1)
        case "--field": o.field = value()
        case "--box": o.boxes.append(value())
        case "--no-real-data": o.noRealData = value()
        case "--note": o.note = value()
        case "--at": o.at = value()
        default: o.positional.append(raw[i])
        }
        i += 1
    }
    return o
}

/// Presses it; with `soft`, a control that is there but disabled is
/// reported (false) instead of stopping.
@discardableResult
func press(_ pid: pid_t, _ title: String, within: [String], row: String? = nil, region: String? = nil, soft: Bool = false) -> Bool {
    say("» press \"\(title)\"" + (row.map { " --row \"\($0)\"" } ?? "") + (region.map { " --region \"\($0)\"" } ?? "")
        + within.map { " --in \"\($0)\"" }.joined())
    if row != nil && region != nil { stop(2, "press takes --row or --region, not both") }
    // One atomic action: after a pause, found again from the start (the
    // page may have changed meanwhile).
    while true {
        let nodes = tree(pid)
        let candidates = controls(nodes, titled: title, roles: pressRoles)
        let (chosen, why) = row.map { pickInRow(nodes, candidates, title: title, row: $0) }
            ?? region.map { pickInRegion(nodes, candidates, title: title, region: $0) }
            ?? pick(nodes, candidates, within: within)
        guard let i = chosen else { stop(4, "no single control titled exactly \"\(title)\": \(why). Nothing pressed. (`buttons` lists them.)") }
        let e = nodes[i].element
        if soft, (attr(e, kAXEnabledAttribute) as? Bool) == false {
            say("  \"\(title)\" is there but disabled: not pressed")
            return false
        }
        let point = pointFor(pid, e)
        if guarded(pid, at: point) == .restart { continue }
        let result = AXUIElementPerformAction(e, kAXPressAction as CFString)
        guard result == .success else { stop(4, "\"\(title)\" didn't take the press (\(result.rawValue))") }
        usleep(300_000)
        return true
    }
}

func focusField(_ pid: pid_t, _ title: String, within: [String]) {
    say("» focus \"\(title)\"" + within.map { " --in \"\($0)\"" }.joined())
    while true {
        let nodes = tree(pid)
        let (chosen, why) = pick(nodes, controls(nodes, titled: title, roles: fieldRoles), within: within)
        guard let i = chosen else { stop(4, "no single field titled exactly \"\(title)\": \(why)") }
        if guarded(pid, at: pointFor(pid, nodes[i].element)) == .restart { continue }
        AXUIElementSetAttributeValue(nodes[i].element, kAXFocusedAttribute as CFString, kCFBooleanTrue)
        usleep(200_000)
        return
    }
}

/// AI Juggler's terminal input: xterm's text area, the one whose
/// description names the terminal, else the only text area in the region.
func focusTerminal(_ pid: pid_t, within: [String]) {
    say("» focus-terminal" + within.map { " --in \"\($0)\"" }.joined())
    while true {
        let nodes = tree(pid)
        let areas = nodes.indices.filter { nodes[$0].role == "AXTextArea" && (nodes[$0].label.lowercased().contains("terminal")) }
        let fallback = nodes.indices.filter { nodes[$0].role == "AXTextArea" }
        var (chosen, why) = pick(nodes, areas, within: within)
        if chosen == nil, areas.isEmpty { (chosen, why) = pick(nodes, fallback, within: within) }
        guard let i = chosen else { stop(4, "no single terminal input: \(why)") }
        if guarded(pid, at: pointFor(pid, nodes[i].element)) == .restart { continue }
        AXUIElementSetAttributeValue(nodes[i].element, kAXFocusedAttribute as CFString, kCFBooleanTrue)
        usleep(200_000)
        return
    }
}

func waitFor(_ pid: pid_t, _ needle: String, timeout: Double, ocr: Bool, gone: Bool) {
    say("» wait \"\(needle)\"" + (gone ? " --gone" : "") + (ocr ? " --ocr" : "") + " --timeout \(Int(timeout))")
    let start = Date()
    var lastOcr = Date.distantPast
    var toldOffScreen = false
    while true {
        let useOcr = ocr && Date().timeIntervalSince(lastOcr) > 1.5
        if useOcr { lastOcr = Date() }
        if useOcr, !toldOffScreen, devWindow(pid) == nil {
            say("  \(describe(.offScreen)): its picture can't be read until it's back")
            toldOffScreen = true
        }
        let seen = shows(screenWords(pid, ocr: useOcr), needle)
        if seen != gone {
            say("  \(gone ? "gone" : "shown") after \(Int(Date().timeIntervalSince(start) * 1000)) ms")
            return
        }
        if Date().timeIntervalSince(start) > timeout { stop(5, "\"\(needle)\" \(gone ? "still shows" : "didn't show") in \(Int(timeout)) s") }
        usleep(400_000)
    }
}

func listTexts(_ pid: pid_t, within: [String], pressable: Bool) {
    let nodes = tree(pid)
    var keep = Set(nodes.indices)
    if let within = within.first {
        // The region around the first node with the words.
        let marks = nodes.indices.filter { nodes[$0].words.contains(within) }
        guard let mark = marks.first else { stop(4, "no words \"\(within)\" on screen") }
        // Widen to the first ancestor that holds a control, so a row's
        // buttons show with its words.
        let ups = ancestors(nodes, mark)
        let root = ups.first(where: { a in descendants(nodes, a).contains { pressRoles.contains(nodes[$0].role) } }) ?? ups.last!
        keep = Set([root] + descendants(nodes, root))
    }
    for i in nodes.indices where keep.contains(i) {
        let n = nodes[i]
        if pressable {
            guard pressRoles.contains(n.role) || fieldRoles.contains(n.role) else { continue }
            let title = n.label.isEmpty ? descendants(nodes, i, levels: 3).filter { nodes[$0].role == "AXStaticText" }.map { nodes[$0].words }.joined(separator: " ") : n.label
            show("\(n.role)\t\"\(title)\"")
        } else if !n.words.isEmpty || !n.value.isEmpty {
            show("\(String(repeating: " ", count: min(n.depth, 30)))\(n.role)\t\(n.label)\t\(n.value)")
        }
    }
}

func shot(_ pid: pid_t, _ o: Opts) {
    guard let name = o.positional.first else { stop(2, "shot NAME (--box X,Y,W,H … | --no-real-data REASON)") }
    guard !o.boxes.isEmpty || o.noRealData != nil else { stop(2, "shot needs --box X,Y,W,H or --no-real-data REASON (the redaction tool fails closed)") }
    let repo = URL(fileURLWithPath: env["AIJ_REPO"] ?? FileManager.default.currentDirectoryPath)
    let tool = repo.appendingPathComponent("scripts/qa/redact_capture.swift")
    guard FileManager.default.fileExists(atPath: tool.path) else { stop(2, "run from the repo (or set AIJ_REPO): \(tool.path) isn't there") }
    try? FileManager.default.createDirectory(at: outDir, withIntermediateDirectories: true)
    let raw = FileManager.default.temporaryDirectory.appendingPathComponent("lab_drive-raw-\(getpid()).png")
    defer { try? FileManager.default.removeItem(at: raw) }
    _ = windowOnScreen(pid) // off screen: waits (or, with --may-activate, brings it back once)
    guard capture(pid, to: raw) else { stop(4, "Dev's window couldn't be captured") }
    let out = outDir.appendingPathComponent(name.hasSuffix(".jpg") ? name : "\(name).jpg")
    var redact = [tool.path, raw.path, out.path]
    for b in o.boxes { redact += ["--box", b] }
    if let reason = o.noRealData { redact += ["--no-real-data", reason] }
    if let note = o.note { redact += ["--note", note] }
    let task = Process()
    task.executableURL = URL(fileURLWithPath: "/usr/bin/swift")
    task.arguments = redact
    try? task.run()
    task.waitUntilExit()
    guard task.terminationStatus == 0 else { stop(4, "the redaction tool refused; nothing kept") }
}

// MARK: - the run sheet's steps

/// Home's Needs-you row for App `app` (its full name, "Kimi Code CLI ·
/// lab A"), and its button: the names Dev gives them (AJ-E1612).
func signInRow(_ app: String) -> String { "\(squash(app)) needs its sign-in" }
func signInButton(_ app: String) -> String { "Sign in to \(squash(app))" }

func signIn(_ pid: pid_t, app: String) {
    press(pid, signInButton(app), within: [], row: signInRow(app))
    waitFor(pid, "Sign in ·", timeout: 20, ocr: false, gone: false)
    say("the sign-in terminal is open: answer its menu with `key`/`type`, read it with `read --ocr`")
}

enum SignInState: String { case notSignedIn = "not-signed-in", signedIn = "signed-in" }

/// What `read-label` found (pure: tested by --selftest): a state with the
/// evidence it rests on, or why it can't tell; and the App page's own
/// sign-in lines, verbatim.
struct SignInReading {
    var state: SignInState?
    var evidence: String
    var appPage: [String] = []
}

/// App `app`'s sign-in state, only from what Dev derives from its sign-in
/// (never from words like "Nothing open", which say no session is open):
///   1. Home's Needs-you row named "<app> needs its sign-in" is there:
///      not signed in;
///   2. Home is on screen with its Needs you read in full, and that row
///      isn't there: signed in, by its absence (Dev reads an App's sign-in
///      again when its sign-in terminal ends, so read this after the end);
///   3. an App page headed `app` that shows a sign-in line: reported
///      verbatim beside either.
func signInState(_ nodes: [Node], app: String) -> SignInReading {
    let row = signInRow(app)
    // 3. The App page: the region under a heading that is the App's name.
    var page: [String] = []
    for h in nodes.indices where nodes[h].role == "AXHeading" && nodes[h].words == squash(app) {
        for a in ancestors(nodes, h).dropFirst().prefix(4) {
            let lines = descendants(nodes, a).map { nodes[$0].words }.filter { $0.lowercased().contains("sign") && $0 != row }
            if !lines.isEmpty { page = lines; break }
        }
    }
    var dedup = Set<String>()
    page = page.filter { dedup.insert($0).inserted }
    // 1. Its row on Home's Needs you (or anywhere: the row's name is its own).
    if nodes.contains(where: { rowRoles.contains($0.role) && $0.label == row }) {
        return SignInReading(state: .notSignedIn, evidence: "Home's Needs you has the row \"\(row)\"", appPage: page)
    }
    // 2. Its absence, only from a Home that shows every need.
    let home = nodes.contains { $0.role == "AXWindow" && ($0.label == "AI Juggler · Home" || $0.label.hasSuffix(" · Home")) }
    guard home else {
        return SignInReading(state: nil, evidence: "Home isn't on screen, so its Needs you can't be read (press \"Home\")", appPage: page)
    }
    if nodes.contains(where: { $0.words.hasPrefix("Checking what needs you") }) {
        return SignInReading(state: nil, evidence: "Home's Needs you is still loading", appPage: page)
    }
    if let more = nodes.first(where: { $0.words.range(of: #"^\+\d+ more needs? you$"#, options: .regularExpression) != nil }) {
        return SignInReading(state: nil, evidence: "Home holds needs behind \"\(more.words)\": press it, then read again", appPage: page)
    }
    return SignInReading(state: .signedIn, evidence: "no row \"\(row)\" on Home's Needs you (by its absence; valid after its sign-in terminal ended)", appPage: page)
}

func readLabel(_ pid: pid_t, app: String) {
    say("» read-label \"\(app)\"")
    let reading = signInState(tree(pid), app: app)
    for line in reading.appPage { say("\(app): its App page says \"\(line)\"") }
    guard let state = reading.state else { stop(4, "\"\(app)\"'s sign-in can't be read: \(reading.evidence)") }
    say("EVIDENCE: \(reading.evidence)")
    say("STATE: \(state.rawValue)")
}

/// The App's own New session, its row action in the rail ("New session:
/// Kimi Code CLI · lab A"): one per App, unlike the header's and the
/// rail's foot's "New session" (one name, one sheet; AJ-E1612 dry run 3).
func newSessionButton(_ app: String) -> String { "New session: \(squash(app))" }
/// New session's sheet names the App it starts on.
func changeAppButton(_ app: String) -> String { "Change app: \(squash(app))" }

/// Whether exactly one control has the title now.
func offers(_ pid: pid_t, _ title: String) -> Bool { controls(tree(pid), titled: title, roles: pressRoles).count == 1 }

func newSession(_ pid: pid_t, tool: String, app: String, project: String, labB: Bool = false) {
    say("new session: \(tool) on \(app) in \(project)")
    press(pid, newSessionButton(app), within: [])
    waitFor(pid, changeAppButton(app), timeout: 10, ocr: false, gone: false)
    if !offers(pid, "Start in \(project)") {
        // Another folder chosen: Change folder (its title names that folder).
        let nodes = tree(pid)
        let folder = Set(nodes.indices.filter { pressRoles.contains(nodes[$0].role) && nodes[$0].label.hasPrefix("Change folder") }.map { nodes[$0].label })
        guard folder.count == 1, let title = folder.first else { stop(4, "New session shows \(folder.count) Change folder controls (\(folder.sorted())); nothing more pressed") }
        press(pid, title, within: [])
        waitFor(pid, "Where to work", timeout: 10, ocr: false, gone: false)
        press(pid, project, within: ["Where to work"])
    }
    let started = press(pid, "Start in \(project)", within: [], soft: labB)
    if labB { say(started ? "  Start was pressed: see what Dev shows next" : "  Dev didn't offer Start on lab B") }
}

/// The words the session page's Stop shows, before and while its stop is
/// held (ui/src/shell/pages.tsx InspectorStop).
let stopWords = ["Stop", "Stopping…"]
/// After a stop is sent, Resume takes Stop's place unarmed this long
/// (ui/src/lib/stop.ts RESUME_SETTLE_MS), plus a margin.
let resumeSettle: Double = 2.5

/// Ends the session whose page is open (its words `row` on screen) by its
/// page's own Stop, and waits until it has ended.
func endSession(_ pid: pid_t, row: [String]) {
    say("» end-session" + row.map { " \"\($0)\"" }.joined())
    let words = screenWords(pid, ocr: false)
    for text in row where !shows(words, text) {
        stop(4, "\"\(text)\" isn't on screen: open that session's page first (press its row). Nothing pressed.")
    }
    press(pid, "Stop", within: [])
    let start = Date()
    while true {
        let nodes = tree(pid)
        let left = stopWords.filter { !controls(nodes, titled: $0, roles: pressRoles).isEmpty }
        if left.isEmpty { break }
        if Date().timeIntervalSince(start) > 45 { stop(5, "the session still shows \(left) 45 s after Stop") }
        usleep(500_000)
    }
    say("  ended after \(Int(Date().timeIntervalSince(start))) s: its page offers no Stop now")
    usleep(useconds_t(resumeSettle * 1_000_000))
}

func send(_ pid: pid_t, _ words: String, field: String?) {
    if let field { focusField(pid, field, within: []) } else { focusTerminal(pid, within: []) }
    say("» type \"\(words)\" + return")
    typeText(pid, words)
    pressKey(pid, "return")
}

// MARK: - the selftest (no clicks, no keys)

/// `gate` against a made-up clock and Mac: `inputAt` is when the person
/// last touched it (they keep typing while `busy` holds), `front` whether
/// Dev is frontmost (activating makes it so unless `refuse`), and Dev's
/// window's state: `hidden`, `minimised`, `elsewhere` (another Space); any
/// of them and it is off screen. Bringing it forward mends each, as the
/// real one does (un-hide, un-minimise, activate: the Space follows).
func activationSelftest() -> [String] {
    var failures: [String] = []
    final class World {
        var t = 1000.0
        var inputAt = 1000.0
        var front = false
        var covered = false
        var hidden = false
        var minimised = false
        var elsewhere = false
        var locked = false
        var saver = false
        var refuse = false
        var activations = 0
        var busyFrom = 0.0
        var busyUntil = 0.0
        /// The person's input while busy is keys only (no pointer event).
        var keysOnly = false
        /// The person's last pointer event (besides `inputAt`, which is any
        /// of theirs, keys only when `keysOnly`).
        var pointerAt = -1e9
        /// The driver's own last key, as the HID state counts it.
        var ownAt = -1e9
        /// The HID state counts the driver's sends (dry run 3's Mac).
        var hidCountsOwn = false
        /// Dev has another (small) window on the Space in use, so
        /// activating it doesn't switch Spaces (dry run 3).
        var otherWindowHere = false
        /// Moving the main window here fails too.
        var cantMove = false
        var activateCalls = 0
        var moves = 0
        var ledger = InputLedger()
        /// Changes at a time (the person locks the screen, another app comes forward…), in order.
        var later: [(Double, (World) -> Void)] = []
        var lines: [String] = []
        /// Each line's time.
        var stamps: [Double] = []
    }
    func mac(_ w: World) -> Mac {
        Mac(now: { w.t },
            sleep: {
                w.t += $0
                if w.t >= w.busyFrom && w.t < w.busyUntil { w.inputAt = w.t }
                while let next = w.later.first, next.0 <= w.t { w.later.removeFirst(); next.1(w) }
            },
            probe: {
                let verdict: Verdict = w.hidden || w.minimised || w.elsewhere ? .offScreen
                    : !w.front ? .notFrontmost("Aside") : w.covered ? .covered("Notification Center, layer 23") : .clear
                let person = max(w.inputAt, w.pointerAt)
                let any = w.t - max(person, w.ownAt)
                let pointer = w.t - (w.keysOnly ? w.pointerAt : person)
                let idle = w.ledger.personIdle(any: any, pointer: pointer, now: w.t)
                return Probe(verdict: verdict, idle: idle, locked: w.locked, saver: w.saver, onConsole: true)
            },
            activate: {
                // The real sequence, on a made-up app and window.
                w.activations += 1
                return bringForwardSteps(Forward(
                    hidden: { w.hidden },
                    unhide: { if !w.refuse { w.hidden = false }; return !w.refuse },
                    minimised: { w.minimised },
                    unminimise: { if !w.refuse { w.minimised = false }; return !w.refuse },
                    activate: {
                        w.activateCalls += 1
                        guard !w.refuse else { return false }
                        // Activating switches Spaces only when Dev has no other window here.
                        w.front = true
                        if !w.otherWindowHere { w.elsewhere = false }
                        return true
                    },
                    onThisSpace: { !w.hidden && !w.minimised && !w.elsewhere },
                    moveHere: {
                        w.moves += 1
                        guard !w.refuse && !w.cantMove else { return false }
                        w.minimised = false; w.elsewhere = false
                        return true
                    },
                    settle: { w.t += $0 }))
            },
            log: { w.lines.append($0); w.stamps.append(w.t) })
    }
    /// A step of `n` events as the driver runs one, opportunistic: each
    /// waits at the gate (a pause restarts it), then is sent ("SENT i"),
    /// taking 1 s. Ends early only if the gate stops it.
    func runStep(_ w: World, _ n: Int, step: String = "send", gap: Double = 1) -> GateEnd? {
        var at: Double?
        var i = 0
        while i < n {
            switch gate(step: step, may: true, opportunistic: true, activatedAt: &at, mac: mac(w), limit: 60) {
            case .send:
                w.lines.append("SENT \(i)"); w.stamps.append(w.t); i += 1
                // The driver's key, posted now: the HID state counts it; its ledger too.
                if w.hidCountsOwn { w.ownAt = w.t; w.ledger.noteOwn(w.t) }
                mac(w).sleep(gap)
            case .restart: continue
            case let end: return end
            }
        }
        return nil
    }
    func first(_ w: World, _ prefix: String) -> Int? { w.lines.firstIndex { $0.hasPrefix(prefix) } }
    func count(_ w: World, _ prefix: String) -> Int { w.lines.filter { $0.hasPrefix(prefix) }.count }
    func beats(_ w: World) -> [Double] { w.lines.indices.filter { w.lines[$0].hasPrefix("HEARTBEAT ") }.map { w.stamps[$0] } }
    func spacedOk(_ times: [Double]) -> Bool { zip(times, times.dropFirst()).allSatisfy { $1 - $0 >= heartbeatEvery && $1 - $0 <= heartbeatEvery + 1.5 } }
    // Off by default: never brought forward, however idle; the guard stops.
    do {
        let w = World(); w.inputAt = 0
        var at: Double?
        if gate(step: "press", may: false, activatedAt: &at, mac: mac(w), limit: 60) != .guardStopped("AI Juggler Dev isn't the frontmost app (Aside is)") || w.activations != 0 {
            failures.append("activation: without --may-activate Dev was brought forward")
        }
    }
    // In use for 15 s more: waits, then brings Dev forward once, at 20 s idle, logged; then sends.
    do {
        let w = World(); w.busyUntil = 1015
        var at: Double?
        let end = gate(step: "sign-in", may: true, activatedAt: &at, mac: mac(w), limit: 120)
        let logged = w.lines.filter { $0.hasPrefix("ACTIVATED ") }
        if end != .send || w.activations != 1 || logged.count != 1 { failures.append("activation: once after idle (\(end), \(w.activations))") }
        if let at, at - w.inputAt < minIdle { failures.append("activation: brought forward after only \(at - w.inputAt) s idle") }
        if let line = logged.first, !(line.contains("step=sign-in") && line.contains("idle=2") && line.contains("T")) { failures.append("activation: its log line \(line)") }
        // The next event: nobody touched the Mac, so it sends with no second activation.
        w.t += 3
        if gate(step: "sign-in", may: true, activatedAt: &at, mac: mac(w), limit: 120) != .send || w.activations != 1 { failures.append("activation: a second event") }
        // Then the person moves the mouse: the run stops, nothing re-raised.
        w.inputAt = w.t; w.t += 0.5
        if gate(step: "sign-in", may: true, activatedAt: &at, mac: mac(w), limit: 120) != .inputStopped || w.activations != 1 { failures.append("activation: new input didn't stop the run") }
        if !w.front { failures.append("activation: Dev was moved after new input") }
    }
    // Locked screen, screen saver: never, however idle.
    for (locked, saver) in [(true, false), (false, true)] {
        let w = World(); w.inputAt = 0; w.locked = locked; w.saver = saver
        var at: Double?
        _ = gate(step: "press", may: true, activatedAt: &at, mac: mac(w), limit: 60)
        if w.activations != 0 { failures.append("activation: while \(locked ? "locked" : "the screen saver runs")") }
    }
    // Refused by the system: one try only, then the guard waits and stops.
    do {
        let w = World(); w.inputAt = 0; w.refuse = true
        var at: Double?
        let end = gate(step: "press", may: true, activatedAt: &at, mac: mac(w), limit: 60)
        if w.activations != 1 { failures.append("activation: tried \(w.activations) times") }
        if case .guardStopped = end {} else { failures.append("activation: refused, then \(end)") }
    }
    // Dev in front but a window over it: not Dev's to fix by activating.
    do {
        let w = World(); w.inputAt = 0; w.front = true; w.covered = true
        var at: Double?
        _ = gate(step: "press", may: true, activatedAt: &at, mac: mac(w), limit: 60)
        if w.activations != 0 { failures.append("activation: for a covering window") }
    }
    // Dev's window off screen (AJ-E1612 dry run 2: 45 min with no window on
    // screen, accessibility still reading the page).
    let offScreenWords = "Dev's window is off screen (minimised/hidden/other Space)"
    // Without the switch: says so, waits, and stops; never brought back.
    do {
        let w = World(); w.inputAt = 0; w.minimised = true
        var at: Double?
        let end = gate(step: "press", may: false, activatedAt: &at, mac: mac(w), limit: 60)
        if end != .guardStopped(offScreenWords) || w.activations != 0 || !w.minimised { failures.append("off screen: without --may-activate (\(end), \(w.activations))") }
        if w.lines.filter({ $0.hasPrefix("WAITING: \(offScreenWords). Nothing is sent.") }).count != 1 { failures.append("off screen: its WAITING line \(w.lines)") }
    }
    // With it, hidden and minimised and in use for 10 s more: waits; at 20 s
    // idle, once: un-hides, un-minimises, activates, each its own line, in
    // that order, with the step and the idle time; then sends.
    do {
        let w = World(); w.hidden = true; w.minimised = true; w.busyUntil = 1010
        var at: Double?
        let end = gate(step: "read-label", may: true, activatedAt: &at, mac: mac(w), limit: 120)
        let raised = w.lines.filter { $0.hasPrefix("UNHID ") || $0.hasPrefix("UNMINIMISED ") || $0.hasPrefix("ACTIVATED ") }
        if end != .send || w.activations != 1 { failures.append("off screen: brought back once (\(end), \(w.activations))") }
        if raised.map({ String($0.prefix(while: { $0 != " " })) }) != ["UNHID", "UNMINIMISED", "ACTIVATED"] { failures.append("off screen: its lines \(raised)") }
        if !raised.allSatisfy({ $0.contains("step=read-label") && $0.contains("idle=2") && $0.contains("T") }) { failures.append("off screen: a line without its step or idle time \(raised)") }
        if let at, at - w.inputAt < minIdle { failures.append("off screen: brought back after only \(at - w.inputAt) s idle") }
        if !w.lines.contains(where: { $0.hasPrefix("WAITING: \(offScreenWords); the Mac is in use") }) { failures.append("off screen: it never said why it waited \(w.lines)") }
        // Then the person touches the Mac: the run stops, nothing re-raised.
        w.inputAt = w.t; w.t += 0.5
        if gate(step: "read-label", may: true, activatedAt: &at, mac: mac(w), limit: 120) != .inputStopped || w.activations != 1 { failures.append("off screen: new input didn't stop the run") }
    }
    // On another Space only: activating alone (no un-hide, no un-minimise).
    do {
        let w = World(); w.inputAt = 0; w.elsewhere = true
        var at: Double?
        let end = gate(step: "press", may: true, activatedAt: &at, mac: mac(w), limit: 60)
        let raised = w.lines.filter { $0.hasPrefix("UNHID ") || $0.hasPrefix("UNMINIMISED ") || $0.hasPrefix("ACTIVATED ") }
        if end != .send || raised.count != 1 || !raised[0].hasPrefix("ACTIVATED ") || w.moves != 0 { failures.append("off screen: another Space (\(end), \(raised))") }
    }
    // Off screen and locked, or the screen saver running: never, however idle.
    for (locked, saver) in [(true, false), (false, true)] {
        let w = World(); w.inputAt = 0; w.hidden = true; w.locked = locked; w.saver = saver
        var at: Double?
        _ = gate(step: "press", may: true, activatedAt: &at, mac: mac(w), limit: 60)
        if w.activations != 0 || !w.hidden { failures.append("off screen: brought back while \(locked ? "locked" : "the screen saver runs")") }
    }
    // Refused: one try, each part logged as refused, then STOPPED at once
    // with what was tried (no wait that can't end).
    do {
        let w = World(); w.inputAt = 0; w.minimised = true; w.refuse = true
        var at: Double?
        let end = gate(step: "press", may: true, activatedAt: &at, mac: mac(w), limit: 60)
        let refused = w.lines.filter { $0.hasSuffix("(the system refused)") }
        if w.activations != 1 || refused.count != 3 { failures.append("off screen: refused (\(w.activations), \(refused))") }
        if end != .guardStopped("\(offScreenWords) after bringing it forward (UNMINIMISED (refused), ACTIVATED (refused), MOVED (refused))") { failures.append("off screen: refused, then \(end)") }
        if count(w, "STOPPED ") != 1 || count(w, "WAITING") != 0 { failures.append("off screen: refused, its STOPPED line \(w.lines)") }
    }
    // AJ-E1612 dry run 3: the main window on another Space while a small
    // Dev window sits on this one, so activating Dev switches no Space.
    // Activated, still not here: MOVED (minimised and restored here), then
    // sent; one bring-forward, one move.
    do {
        let w = World(); w.inputAt = 0; w.elsewhere = true; w.otherWindowHere = true
        var at: Double?
        let end = gate(step: "press", may: true, activatedAt: &at, mac: mac(w), limit: 60)
        let raised = w.lines.filter { $0.hasPrefix("ACTIVATED ") || $0.hasPrefix("MOVED ") }.map { String($0.prefix(while: { $0 != " " })) }
        if end != .send || raised != ["ACTIVATED", "MOVED"] || w.activations != 1 || w.moves != 1 { failures.append("another Space: moved here (\(end), \(raised), \(w.moves))") }
        if w.lines.contains(where: { $0.hasPrefix("MOVED ") && $0.hasSuffix("refused)") }) { failures.append("another Space: MOVED said refused") }
    }
    // …and when it can't be moved either: STOPPED at once, saying so; never
    // the hours of waiting dry run 3 spent (opportunistic or not).
    for opp in [false, true] {
        let w = World(); w.inputAt = 0; w.elsewhere = true; w.otherWindowHere = true; w.cantMove = true
        var at: Double?
        let start = w.t
        let end = gate(step: "press", may: true, opportunistic: opp, activatedAt: &at, mac: mac(w), limit: 600)
        if end != .guardStopped("\(offScreenWords) after bringing it forward (ACTIVATED, MOVED (refused))") { failures.append("another Space, can't move: \(end)") }
        if w.t - start > 10 || w.activations != 1 || count(w, "HEARTBEAT") != 0 { failures.append("another Space, can't move: took \(w.t - start) s, \(w.activations) tries") }
        guard let line = w.lines.first(where: { $0.hasPrefix("STOPPED ") }) else { failures.append("another Space, can't move: no STOPPED line"); continue }
        if !(line.contains("step=press") && line.contains("Nothing was sent") && line.contains("yourself")) { failures.append("another Space, can't move: its line \(line)") }
    }
    // OPPORTUNISTIC. Input mid-step (the person at the Mac from 1001.5 to
    // 1010): PAUSED, nothing sent while paused, Dev not moved; after 20 s
    // without input, RESUMED, and the same step goes on to its end.
    do {
        let w = World(); w.inputAt = 0; w.front = true; w.busyFrom = 1001.5; w.busyUntil = 1010
        let end = runStep(w, 5)
        if end != nil || count(w, "SENT ") != 5 { failures.append("opportunistic: the step didn't go on after the pause (\(String(describing: end)), \(w.lines))") }
        if count(w, "PAUSED ") != 1 || count(w, "RESUMED ") != 1 { failures.append("opportunistic: PAUSED/RESUMED lines \(w.lines)") }
        if let p = first(w, "PAUSED "), let r = first(w, "RESUMED "), p < r {
            if !(w.lines[p].contains("step=send") && w.lines[p].contains("idle=0.0s") && w.lines[p].contains("T")) { failures.append("opportunistic: the PAUSED line \(w.lines[p])") }
            if !(w.lines[r].contains("step=send") && w.lines[r].contains("T")) { failures.append("opportunistic: the RESUMED line \(w.lines[r])") }
            if w.stamps[r] - w.inputAt < minIdle { failures.append("opportunistic: resumed after only \(w.stamps[r] - w.inputAt) s idle") }
            let sentInUse = w.lines.indices.filter { w.lines[$0].hasPrefix("SENT ") && w.stamps[$0] >= 1002 && w.stamps[$0] < w.stamps[r] }
            if !sentInUse.isEmpty { failures.append("opportunistic: sent while paused \(sentInUse.map { w.lines[$0] })") }
        } else {
            failures.append("opportunistic: no PAUSED then RESUMED \(w.lines)")
        }
        if w.activations != 0 || !w.front || count(w, "STOPPED") != 0 { failures.append("opportunistic: Dev was moved, or the step stopped, during a pause") }
    }
    // The person left another app in front: after RESUMED the guard is
    // checked again (it waits), then Dev is brought forward once, then sent.
    do {
        let w = World(); w.inputAt = 0; w.front = true; w.busyFrom = 1001.5; w.busyUntil = 1010
        w.later = [(1005, { $0.front = false })]
        let end = runStep(w, 3)
        if end != nil || w.activations != 1 { failures.append("opportunistic: resumed behind another app (\(String(describing: end)), \(w.activations))") }
        if let r = first(w, "RESUMED "), let a = first(w, "ACTIVATED "), let s = first(w, "SENT 2"), r < a, a < s {
            if a != r + 1 { failures.append("opportunistic: not brought forward straight after RESUMED (\(w.lines[r + 1]))") }
            if w.stamps[a] - w.inputAt < minIdle { failures.append("opportunistic: brought forward after only \(w.stamps[a] - w.inputAt) s idle") }
        } else {
            failures.append("opportunistic: not RESUMED, ACTIVATED, SENT in that order \(w.lines)")
        }
    }
    // The person left a window over Dev's: after RESUMED the full guard is
    // checked again and nothing is sent until it is gone (not Dev's to fix:
    // no activation).
    do {
        let w = World(); w.inputAt = 0; w.front = true; w.busyFrom = 1001.5; w.busyUntil = 1010
        w.later = [(1005, { $0.covered = true }), (1100, { $0.covered = false })]
        let end = runStep(w, 3)
        if end != nil || w.activations != 0 { failures.append("opportunistic: resumed under a window (\(String(describing: end)), \(w.activations))") }
        if let r = first(w, "RESUMED "), let s = first(w, "SENT 2"), r < s {
            if !w.lines[r + 1].hasPrefix("WAITING: another window is on top at the point") { failures.append("opportunistic: the guard wasn't checked again after RESUMED (\(w.lines[r + 1]))") }
            if w.stamps[s] < 1100 { failures.append("opportunistic: sent under a window after RESUMED") }
        } else {
            failures.append("opportunistic: not RESUMED then SENT \(w.lines)")
        }
    }
    // Two pauses: one activation per resume at most; the second refused:
    // STOPPED at once with what was tried, not tried again, no wait (dry
    // run 3 waited hours behind an activation that did nothing).
    do {
        let w = World(); w.inputAt = 0; w.front = true; w.busyFrom = 1001.5; w.busyUntil = 1010
        w.later = [(1005, { $0.front = false }),
                   (1040, { $0.busyFrom = 1040; $0.busyUntil = 1045; $0.front = false; $0.refuse = true })]
        let end = runStep(w, 60)
        if case .guardStopped = end {} else { failures.append("opportunistic: a refused bring-forward didn't stop (\(String(describing: end)))") }
        if count(w, "PAUSED ") != 2 || count(w, "RESUMED ") != 2 || w.activations != 2 { failures.append("opportunistic: two resumes, \(w.activations) activations") }
        if count(w, "ACTIVATED ") != 2 || w.lines.filter({ $0.hasPrefix("ACTIVATED ") && $0.hasSuffix("(the system refused)") }).count != 1 { failures.append("opportunistic: a refused activation tried again") }
        if count(w, "STOPPED ") != 1 || count(w, "HEARTBEAT") != 0 || count(w, "SENT ") >= 60 { failures.append("opportunistic: after the refused one \(w.lines.suffix(4))") }
    }
    // Paused, then the screen locks (or the screen saver starts) with
    // another app in front: no RESUMED and no activation until it is
    // unlocked 2 h later, however idle; heartbeats every 10 min meanwhile.
    for saver in [false, true] {
        let w = World(); w.inputAt = 0; w.front = true; w.busyFrom = 1001.5; w.busyUntil = 1010
        let unlock = 1000.0 + 7200
        w.later = [(1005, { $0.front = false; if saver { $0.saver = true } else { $0.locked = true } }),
                   (unlock, { $0.locked = false; $0.saver = false })]
        let end = runStep(w, 3)
        let what = saver ? "the screen saver runs" : "the screen is locked"
        if end != nil || w.activations != 1 { failures.append("opportunistic, \(what): \(String(describing: end)), \(w.activations) activations") }
        if let r = first(w, "RESUMED "), w.stamps[r] < unlock { failures.append("opportunistic: resumed while \(what)") }
        if let a = first(w, "ACTIVATED "), w.stamps[a] < unlock { failures.append("opportunistic: brought forward while \(what)") }
        let paused = beats(w)
        if let p = first(w, "PAUSED ") {
            if paused.count < 10 || !spacedOk(paused) || (paused.first.map { $0 - w.stamps[p] < heartbeatEvery } ?? true) {
                failures.append("opportunistic: heartbeats while paused (\(paused.count), spaced \(spacedOk(paused)))")
            }
        }
        if !w.lines.contains(where: { $0.hasPrefix("HEARTBEAT ") && $0.contains("step=send paused") && $0.hasSuffix(what) }) { failures.append("opportunistic: a heartbeat saying \(what)") }
    }
    // No input at all, but locked with another app in front: never brought
    // forward while locked, however long; once unlocked, once.
    do {
        let w = World(); w.inputAt = 0; w.locked = true
        let unlock = 1000.0 + 7200
        w.later = [(unlock, { $0.locked = false })]
        let end = runStep(w, 1)
        if end != nil || w.activations != 1 || count(w, "PAUSED ") != 0 { failures.append("opportunistic, locked without input: \(String(describing: end)), \(w.activations)") }
        if let a = first(w, "ACTIVATED "), w.stamps[a] < unlock { failures.append("opportunistic: brought forward while locked") }
    }
    // A window over Dev's for 3 h (not Dev's to fix): opportunistic waits
    // it out; without the flag, the same stops after AIJ_DRIVE_WAIT (unchanged).
    do {
        let w = World(); w.inputAt = 0; w.front = true; w.covered = true
        w.later = [(1000 + 3 * 3600, { $0.covered = false })]
        var at: Double?
        if gate(step: "press", may: true, opportunistic: true, activatedAt: &at, mac: mac(w), limit: 60) != .send || w.activations != 0 { failures.append("opportunistic: a covering window stopped it") }
        if !spacedOk(beats(w)) || beats(w).count < 17 { failures.append("opportunistic: heartbeats over 3 h (\(beats(w).count))") }
        let v = World(); v.inputAt = 0; v.front = true; v.covered = true
        var vat: Double?
        if case .guardStopped = gate(step: "press", may: true, activatedAt: &vat, mac: mac(v), limit: 60) {} else { failures.append("without --opportunistic the limit went") }
        if count(v, "HEARTBEAT ") != 0 || count(v, "PAUSED ") != 0 { failures.append("without --opportunistic: heartbeat or pause lines") }
    }
    // AJ-E1612 dry run 3: the HID state counts the driver's own keys. A
    // send of 24 characters (one key each, 0.1 s apart) with nobody at the
    // Mac: no PAUSED, all sent at once (it paused 20 s per key).
    do {
        let w = World(); w.inputAt = 0; w.front = true; w.hidCountsOwn = true
        let end = runStep(w, 24, gap: keyGap)
        if end != nil || count(w, "SENT ") != 24 || count(w, "PAUSED ") != 0 { failures.append("own keys: paused the step (\(count(w, "PAUSED ")) PAUSED)") }
        if let last = w.stamps.last, last - 1000 > 24 * keyGap + 1 { failures.append("own keys: 24 keys took \(last - 1000) s") }
    }
    // The same without the ledger (dry run 3's driver): every key pauses.
    do {
        let w = World(); w.inputAt = 0; w.front = true; w.hidCountsOwn = true
        var at: Double?
        _ = gate(step: "send", may: true, opportunistic: true, activatedAt: &at, mac: mac(w), limit: 60)
        w.ownAt = w.t // a key the ledger never heard of: the person's, as far as it knows
        if case .pause = decide(mac(w).probe(), may: true, activatedAt: nil, now: w.t, opportunistic: true) {} else { failures.append("own keys: a key not in the ledger didn't pause") }
    }
    // The person types (keys only, no pointer) mid-send: PAUSED, nothing
    // sent while they type, RESUMED 20 s after their last key, then the rest.
    do {
        let w = World(); w.inputAt = 0; w.front = true; w.hidCountsOwn = true
        w.keysOnly = true; w.busyFrom = 1000.55; w.busyUntil = 1003
        let end = runStep(w, 24, gap: keyGap)
        if end != nil || count(w, "SENT ") != 24 { failures.append("person's keys: the step didn't finish (\(String(describing: end)))") }
        if count(w, "PAUSED ") < 1 { failures.append("person's keys: no PAUSED while they typed \(w.lines.prefix(12))") }
        if let r = first(w, "RESUMED "), w.stamps[r] - w.inputAt < minIdle { failures.append("person's keys: resumed after only \(w.stamps[r] - w.inputAt) s") }
        let sentWhileTyping = w.lines.indices.filter { w.lines[$0].hasPrefix("SENT ") && w.stamps[$0] > 1000.7 && w.stamps[$0] < 1003 }
        if !sentWhileTyping.isEmpty { failures.append("person's keys: sent while they typed \(sentWhileTyping.map { w.lines[$0] })") }
    }
    // The person moves the mouse mid-send: PAUSED (pointer events are never the driver's).
    do {
        let w = World(); w.inputAt = 0; w.front = true; w.hidCountsOwn = true
        w.later = [(1000.5, { $0.pointerAt = $0.t })]
        _ = runStep(w, 24, gap: keyGap)
        if count(w, "PAUSED ") != 1 { failures.append("pointer: a mouse move mid-send didn't pause it once (\(count(w, "PAUSED ")))") }
    }
    // Not opportunistic, after Dev was brought forward: its own keys don't
    // stop the run; the person's next key does.
    do {
        let w = World(); w.inputAt = 0; w.hidCountsOwn = true
        var at: Double?
        for i in 0..<5 {
            if gate(step: "send", may: true, activatedAt: &at, mac: mac(w), limit: 60) != .send { failures.append("own keys after ACTIVATED: key \(i) didn't send") }
            w.ownAt = w.t; w.ledger.noteOwn(w.t); mac(w).sleep(keyGap)
        }
        if w.activations != 1 { failures.append("own keys after ACTIVATED: \(w.activations) activations") }
        w.inputAt = w.t; w.t += 0.3
        if gate(step: "send", may: true, activatedAt: &at, mac: mac(w), limit: 60) != .inputStopped { failures.append("own keys after ACTIVATED: the person's key didn't stop it") }
    }
    // The run's log: typed words by their count only, every line stamped.
    if logged("» type \"reply with the word lab\" + return", at: 0) != "1970-01-01T00:00:00Z » type (23 characters) + return" { failures.append("log: a send's line \(logged("» type \"reply with the word lab\" + return", at: 0))") }
    if logged("» type \"2\"", at: 0) != "1970-01-01T00:00:00Z » type (1 character)" { failures.append("log: a type's line \(logged("» type \"2\"", at: 0))") }
    if logged("PAUSED x", at: 0) != "1970-01-01T00:00:00Z PAUSED x" { failures.append("log: a plain line") }
    if runLine(["send", "reply with the word lab", "--field", "Message"]) != "RUN send (23 characters)" || runLine(["send", "reply with the word lab"]) != "RUN send (23 characters)" { failures.append("log: the send command line \(runLine(["send", "reply with the word lab"]))") }
    if runLine(["type", "secret"]).contains("secret") { failures.append("log: typed words in the command line") }
    if runLine(["press", "Sign in to Kimi Code CLI · lab A", "--row", "x"]) != "RUN press \"Sign in to Kimi Code CLI · lab A\" --row x" { failures.append("log: a press's command line \(runLine(["press", "Sign in to Kimi Code CLI · lab A", "--row", "x"]))") }
    // The run's name: given (cleaned), else Dev's launch and pid.
    if runName(given: "dry-run 4/x", pid: 7, launched: nil) != "dry-run4x" { failures.append("run name: given \(runName(given: "dry-run 4/x", pid: 7, launched: nil))") }
    if runName(given: nil, pid: 80373, launched: Date(timeIntervalSince1970: 1_795_000_000)) != "dev80373-20261118T110640Z" {
        failures.append("run name: from Dev \(runName(given: nil, pid: 80373, launched: Date(timeIntervalSince1970: 1_795_000_000)))")
    }
    // The ledger itself.
    do {
        var l = InputLedger()
        if l.personIdle(any: 300, pointer: 300, now: 2000) != 300 || l.personAt != 1700 { failures.append("ledger: the first reading") }
        l.noteOwn(2000)
        if l.personIdle(any: 0.02, pointer: 300.02, now: 2000.02) != 300.02 { failures.append("ledger: its own key counted") }
        if l.personIdle(any: 0.01, pointer: 300.2, now: 2000.2) != 0.01 || l.personAt != 2000.19 { failures.append("ledger: a key 0.19 s after its own wasn't the person's") }
        var m = InputLedger(); m.noteOwn(2000)
        if m.personIdle(any: 0.02, pointer: 50, now: 2000.02) != 0.02 { failures.append("ledger: nothing seen of the person: not the old reading") }
        var n = InputLedger(own: [], personAt: 1000); n.noteOwn(2000)
        if n.personIdle(any: 0.01, pointer: 3, now: 2000.01) != 3 { failures.append("ledger: a pointer event under its own key") }
        n.noteOwn(2200)
        if n.own != [2200] { failures.append("ledger: old posts kept \(n.own)") }
        if InputLedger(own: [10], personAt: nil).isOwn(10 + ownAfter + 0.001) || !InputLedger(own: [10], personAt: nil).isOwn(10 - ownBefore) { failures.append("ledger: its window") }
        let coded = try? JSONDecoder().decode(InputLedger.self, from: JSONEncoder().encode(n))
        if coded != n { failures.append("ledger: kept and read back") }
    }
    // The pure rule, opportunistic.
    let clearNow = Probe(verdict: .clear, idle: 19.9, locked: false, saver: false, onConsole: true)
    if decide(clearNow, may: true, activatedAt: nil, now: 0, opportunistic: true) != .pause { failures.append("decide: opportunistic at 19.9 s idle") }
    if decide(clearNow, may: true, activatedAt: nil, now: 0) != .send { failures.append("decide: without the flag, input doesn't pause") }
    var clearIdle = clearNow; clearIdle.idle = 20
    if decide(clearIdle, may: true, activatedAt: nil, now: 0, opportunistic: true) != .send { failures.append("decide: opportunistic at 20 s idle") }
    // Input 30 s after an activation, gone 20 s: opportunistic sends; without the flag, stops.
    var later = clearIdle; later.idle = 25
    if decide(later, may: true, activatedAt: 0, now: 55, opportunistic: true) != .send { failures.append("decide: opportunistic after old input") }
    if decide(later, may: true, activatedAt: 0, now: 55) != .stopNewInput { failures.append("decide: without the flag, input after an activation stops") }
    if pauseHolds(clearIdle) != nil { failures.append("pauseHolds: 20 s idle, unlocked") }
    for (field, want) in [("locked", "the screen is locked"), ("saver", "the screen saver runs"), ("console", "this login isn't the one on screen")] {
        var q = clearIdle
        if field == "locked" { q.locked = true } else if field == "saver" { q.saver = true } else { q.onConsole = false }
        if pauseHolds(q) != want { failures.append("pauseHolds: \(field)") }
    }
    if pauseHolds(clearNow) == nil { failures.append("pauseHolds: 19.9 s idle") }
    // The pure rule at its edges.
    let quiet = Probe(verdict: .notFrontmost("Aside"), idle: 19.9, locked: false, saver: false, onConsole: true)
    if decide(quiet, may: true, activatedAt: nil, now: 0) == .activate { failures.append("decide: 19.9 s idle") }
    var twenty = quiet; twenty.idle = 20
    if decide(twenty, may: true, activatedAt: nil, now: 0) != .activate { failures.append("decide: 20 s idle") }
    var away = twenty; away.onConsole = false
    if decide(away, may: true, activatedAt: nil, now: 0) == .activate { failures.append("decide: off console") }
    var gone = twenty; gone.verdict = .offScreen
    if decide(gone, may: true, activatedAt: nil, now: 0) != .activate { failures.append("decide: off screen at 20 s idle") }
    if decide(gone, may: false, activatedAt: nil, now: 0) != .wait(offScreenWords) { failures.append("decide: off screen without the switch") }
    gone.idle = 19.9
    if decide(gone, may: true, activatedAt: nil, now: 0) == .activate { failures.append("decide: off screen at 19.9 s idle") }
    return failures
}

func selftest() {
    let dev: pid_t = 4242
    let devWin = Win(owner: dev, ownerName: "AI Juggler Dev", title: "AI Juggler · Home", layer: 0, alpha: 1,
                     bounds: CGRect(x: 100, y: 100, width: 800, height: 600))
    let other = Win(owner: 77, ownerName: "Another App", title: "Notes", layer: 0, alpha: 1,
                    bounds: CGRect(x: 600, y: 400, width: 400, height: 400))
    let banner = Win(owner: 88, ownerName: "Notification Center", title: "", layer: 23, alpha: 1,
                     bounds: CGRect(x: 500, y: 100, width: 380, height: 90))
    let ghost = Win(owner: 99, ownerName: "Overlay", title: "", layer: 25, alpha: 0, bounds: CGRect(x: 0, y: 0, width: 2000, height: 2000))
    let inside = CGPoint(x: 200, y: 200)
    var failures: [String] = []
    func expect(_ got: Verdict, _ want: Verdict, _ what: String) {
        if got != want { failures.append("\(what): got \(describe(got)), want \(describe(want))") }
    }
    // Dev in front, its window on top: clear.
    expect(judge(dev: dev, frontmost: dev, frontmostName: "AI Juggler Dev", windows: [ghost, devWin, other], point: inside), .clear, "front and on top")
    // Another app in front: nothing is sent, wherever the point is.
    expect(judge(dev: dev, frontmost: 77, frontmostName: "Another App", windows: [devWin], point: inside),
           .notFrontmost("Another App"), "another app in front")
    // Dev in front, but no frontmost app known: refused.
    expect(judge(dev: dev, frontmost: nil, frontmostName: "", windows: [devWin], point: inside), .notFrontmost("another app"), "no frontmost")
    // Dev in front, another window over the point (a notification banner): refused.
    expect(judge(dev: dev, frontmost: dev, frontmostName: "AI Juggler Dev", windows: [banner, devWin], point: CGPoint(x: 600, y: 150)),
           .covered("Notification Center, layer 23"), "banner over the point")
    // …the same banner elsewhere doesn't matter.
    expect(judge(dev: dev, frontmost: dev, frontmostName: "AI Juggler Dev", windows: [banner, devWin], point: inside), .clear, "banner elsewhere")
    // Another app's window above Dev's at the point (Dev still frontmost).
    expect(judge(dev: dev, frontmost: dev, frontmostName: "AI Juggler Dev", windows: [other, devWin], point: CGPoint(x: 700, y: 500)),
           .covered("Another App (Notes), layer 0"), "window above")
    // The point outside every window.
    expect(judge(dev: dev, frontmost: dev, frontmostName: "AI Juggler Dev", windows: [devWin], point: CGPoint(x: 5000, y: 5000)),
           .offWindow, "off window")
    // A same-named app with another pid isn't Dev.
    let twin = Win(owner: 4243, ownerName: "AI Juggler Dev", title: "AI Juggler · Home", layer: 0, alpha: 1, bounds: devWin.bounds)
    expect(judge(dev: dev, frontmost: dev, frontmostName: "AI Juggler Dev", windows: [twin, devWin], point: inside),
           .covered("AI Juggler Dev (AI Juggler · Home), layer 0"), "a twin by name")
    expect(judge(dev: dev, frontmost: 4243, frontmostName: "AI Juggler Dev", windows: [devWin], point: inside),
           .notFrontmost("AI Juggler Dev"), "a twin in front")
    // Exact titles in a region: Home's two "lab A needs its sign-in" rows,
    // OpenCode's and Kimi Code's, each with a Sign in (a made-up tree).
    let dummy = AXUIElementCreateApplication(0)
    func n(_ role: String, _ label: String, _ value: String, _ parent: Int?, _ depth: Int) -> Node {
        Node(element: dummy, role: role, label: label, value: value, parent: parent, depth: depth)
    }
    let rows: [Node] = [
        n("AXWindow", "AI Juggler · Home", "", nil, 0),
        n("AXGroup", "", "", 0, 1),                                             // 1: row OpenCode
        n("AXStaticText", "", "lab A needs its sign-in", 1, 2),
        n("AXStaticText", "", "OpenCode's own sign-in, in a terminal here", 1, 2),
        n("AXButton", "", "", 1, 2),                                            // 4
        n("AXStaticText", "", "Sign in", 4, 3),
        n("AXGroup", "", "", 0, 1),                                             // 6: row Kimi
        n("AXStaticText", "", "lab A needs its sign-in", 6, 2),
        n("AXStaticText", "", "Kimi Code's own sign-in, in a terminal here", 6, 2),
        n("AXButton", "Sign in", "", 6, 2),                                     // 9
        n("AXButton", "Sign in to everything", "", 0, 1),                       // 10: not exact
    ]
    let signIns = controls(rows, titled: "Sign in", roles: pressRoles)
    if signIns != [4, 9] { failures.append("exact titles: \(signIns)") }
    // Two controls of one title: refused, whatever words narrow it (--row names the row).
    if pick(rows, signIns, within: ["lab A needs its sign-in", "OpenCode's own sign-in"]).0 != nil { failures.append("--in chose between two of one title") }
    if pick(rows, signIns, within: ["Kimi Code's own sign-in"]).0 != nil { failures.append("--in chose between two of one title (Kimi)") }
    if pick(rows, signIns, within: ["lab A needs its sign-in"]).0 != nil { failures.append("an ambiguous region picked one") }
    // One control of the title: --in still checks its region holds the words.
    let only = controls(rows, titled: "Sign in to everything", roles: pressRoles)
    if pick(rows, only, within: ["lab A needs its sign-in"]).0 != 10 { failures.append("--in: the one control in its region") }
    if pick(rows, only, within: ["Grok Build's own sign-in"]).0 != nil { failures.append("--in: words not on screen") }
    if pick(rows, signIns, within: []).0 != nil { failures.append("two Sign in without a region picked one") }
    if pick(rows, signIns, within: ["Grok Build's own sign-in"]).0 != nil { failures.append("a missing region picked one") }
    if controls(rows, titled: "Sign", roles: pressRoles) != [] { failures.append("a prefix matched") }
    // AJ-E1612, the bug the dry run met: the first row an unnamed box whose
    // button sits beside the rows, the rows' buttons all one name. `--in`
    // picked Kimi Code's button for OpenCode's words.
    let bug: [Node] = [
        n("AXWindow", "AI Juggler · Home", "", nil, 0),
        n("AXGroup", "Needs you", "", 0, 1),                                    // 1
        n("AXStaticText", "", "lab A needs its sign-in", 1, 2),                 // OpenCode's words…
        n("AXStaticText", "", "OpenCode's own sign-in, in a terminal here", 1, 2),
        n("AXList", "", "", 1, 2),                                              // 4
        n("AXGroup", "", "", 4, 3),                                             // 5: Kimi's row
        n("AXStaticText", "", "lab A needs its sign-in", 5, 4),
        n("AXStaticText", "", "Kimi Code's own sign-in, in a terminal here", 5, 4),
        n("AXButton", "Sign in — lab A needs its sign-in", "", 5, 4),           // 8
        n("AXButton", "Sign in — lab A needs its sign-in", "", 1, 2),           // 9: OpenCode's, beside
    ]
    let same = controls(bug, titled: "Sign in — lab A needs its sign-in", roles: pressRoles)
    if same != [8, 9] { failures.append("the bug tree's buttons: \(same)") }
    if pick(bug, same, within: ["lab A needs its sign-in", "OpenCode's own sign-in"]).0 != nil { failures.append("--in tied a button to the wrong row (the dry run's pick)") }
    if pickInRow(bug, same, title: "Sign in — lab A needs its sign-in", row: "Needs you").0 != nil { failures.append("--row accepted a title naming two controls") }
    // Named rows, but a button beside its row (outside it): refused.
    let beside: [Node] = [
        n("AXWindow", "AI Juggler · Home", "", nil, 0),
        n("AXGroup", "OpenCode CLI · lab A needs its sign-in", "", 0, 1),     // 1
        n("AXStaticText", "", "lab A needs its sign-in", 1, 2),
        n("AXList", "", "", 0, 1),                                              // 3
        n("AXGroup", "Kimi Code CLI · lab A needs its sign-in", "", 3, 2),    // 4
        n("AXButton", "Sign in to Kimi Code CLI · lab A", "", 4, 3),           // 5
        n("AXButton", "Sign in to OpenCode CLI · lab A", "", 3, 2),            // 6: a sibling of Kimi's row
    ]
    let open = controls(beside, titled: "Sign in to OpenCode CLI · lab A", roles: pressRoles)
    if pickInRow(beside, open, title: "Sign in to OpenCode CLI · lab A", row: "OpenCode CLI · lab A needs its sign-in").0 != nil {
        failures.append("--row pressed a button outside its row")
    }
    let kimiButton = controls(beside, titled: "Sign in to Kimi Code CLI · lab A", roles: pressRoles)
    if pickInRow(beside, kimiButton, title: "Sign in to Kimi Code CLI · lab A", row: "Kimi Code CLI · lab A needs its sign-in").0 != 5 {
        failures.append("--row missed the button inside its row")
    }
    if pickInRow(beside, kimiButton, title: "Sign in to Kimi Code CLI · lab A", row: "Grok Build CLI · lab A needs its sign-in").0 != nil {
        failures.append("--row pressed with no such row")
    }
    let twoRows = beside + [n("AXGroup", "Kimi Code CLI · lab A needs its sign-in", "", 0, 1)]
    if pickInRow(twoRows, kimiButton, title: "Sign in to Kimi Code CLI · lab A", row: "Kimi Code CLI · lab A needs its sign-in").0 != nil {
        failures.append("--row chose between two rows of one name")
    }
    if signInRow(" Kimi Code CLI ·  lab A") != "Kimi Code CLI · lab A needs its sign-in" || signInButton("Kimi Code CLI · lab A") != "Sign in to Kimi Code CLI · lab A" {
        failures.append("the sign-in names")
    }
    // read-label: only what Dev derives from the sign-in (Home's row, its
    // absence on a fully shown Home, the App page's own line), never "Nothing open".
    let homeNeeds: [Node] = [
        n("AXWindow", "AI Juggler · Home", "", nil, 0),
        n("AXGroup", "Needs you", "", 0, 1),                                   // 1
        n("AXGroup", "Kimi Code CLI · lab A needs its sign-in", "", 1, 2),    // 2
        n("AXStaticText", "", "lab A needs its sign-in", 2, 3),
        n("AXButton", "Sign in to Kimi Code CLI · lab A", "", 2, 3),
        n("AXGroup", "", "", 0, 1),                                            // 5: Your apps
        n("AXStaticText", "", "OpenCode CLI", 5, 2),
        n("AXStaticText", "", "Nothing open", 5, 2),
    ]
    let kimiOut = signInState(homeNeeds, app: "Kimi Code CLI · lab A")
    if kimiOut.state != .notSignedIn || !kimiOut.evidence.contains("has the row") { failures.append("read-label: Home's row means not signed in") }
    let openIn = signInState(homeNeeds, app: "OpenCode CLI · lab A")
    if openIn.state != .signedIn || !openIn.evidence.contains("absence") { failures.append("read-label: no row on a full Home means signed in") }
    if signInState(homeNeeds, app: "Kimi Code CLI · lab B").state != .signedIn { failures.append("read-label: lab B's own row, not lab A's") }
    // Not Home (an Accounts row with "Nothing open"): no state, whatever it shows.
    let accounts: [Node] = [
        n("AXWindow", "AI Juggler · Accounts", "", nil, 0),
        n("AXGroup", "", "", 0, 1),
        n("AXStaticText", "", "Kimi Code CLI", 1, 2),
        n("AXStaticText", "", "Nothing open", 1, 2),
        n("AXButton", "Open Kimi Code CLI · lab A", "", 1, 2),
    ]
    if signInState(accounts, app: "Kimi Code CLI · lab A").state != nil { failures.append("read-label: a state from \"Nothing open\"") }
    // A Home holding needs behind "+n more", or still loading: no state by absence.
    if signInState(homeNeeds + [n("AXButton", "", "+2 more need you", 1, 2)], app: "OpenCode CLI · lab A").state != nil { failures.append("read-label: absence behind +n more") }
    if signInState(homeNeeds + [n("AXHeading", "", "Checking what needs you…", 0, 1)], app: "OpenCode CLI · lab A").state != nil { failures.append("read-label: absence while loading") }
    // The App page's own sign-in line, verbatim, beside the state.
    let appPage = homeNeeds + [
        n("AXGroup", "", "", 0, 1),                                            // 8
        n("AXHeading", "", "OpenCode CLI · lab A", 8, 2),
        n("AXStaticText", "", "Signed in with OpenCode's own sign-in", 8, 2),
        n("AXStaticText", "", "Nothing open", 8, 2),
    ]
    let paged = signInState(appPage, app: "OpenCode CLI · lab A")
    if paged.appPage != ["Signed in with OpenCode's own sign-in"] || paged.state != .signedIn { failures.append("read-label: the App page's line \(paged.appPage)") }
    // AJ-E1612 dry run 3: the header's "Usage" (navigation "Main") and a
    // session page's own tab "Usage": --region "Main" is the header's.
    let usage: [Node] = [
        n("AXWindow", "AI Juggler · lab", "", nil, 0),
        n("AXGroup", "Main", "", 0, 1),                                       // 1: the header's navigation
        n("AXButton", "Usage", "", 1, 2),                                     // 2
        n("AXButton", "History", "", 1, 2),
        n("AXGroup", "Session tabs", "", 0, 1),                               // 4: the page's tabs
        n("AXRadioButton", "Usage", "", 4, 2),                                // 5: before the a11y fix
        n("AXHeading", "Main", "", 0, 1),                                     // a heading of that name isn't a region
    ]
    let usages = controls(usage, titled: "Usage", roles: pressRoles)
    if usages != [2, 5] { failures.append("region: the two Usage controls \(usages)") }
    if pick(usage, usages, within: []).0 != nil { failures.append("region: a bare title chose between two") }
    if pickInRegion(usage, usages, title: "Usage", region: "Main").0 != 2 { failures.append("region: --region Main missed the header's Usage") }
    if pickInRegion(usage, usages, title: "Usage", region: "Session tabs").0 != 5 { failures.append("region: --region Session tabs missed the tab") }
    if pickInRegion(usage, usages, title: "Usage", region: "Sidebar").0 != nil { failures.append("region: no such region, yet one chosen") }
    if pickInRegion(usage + [n("AXGroup", "Main", "", 0, 1)], usages, title: "Usage", region: "Main").0 != nil { failures.append("region: two regions of one name") }
    if pickInRegion(usage + [n("AXButton", "Usage", "", 1, 2)], usages + [7], title: "Usage", region: "Main").0 != nil { failures.append("region: two of the title inside it") }
    // New session: the App's own row action is the key, not the shared title.
    if newSessionButton(" Kimi Code CLI ·  lab A") != "New session: Kimi Code CLI · lab A" || changeAppButton("Kimi Code CLI · lab A") != "Change app: Kimi Code CLI · lab A" {
        failures.append("the new-session names")
    }
    let rail: [Node] = [
        n("AXWindow", "AI Juggler · Home", "", nil, 0),
        n("AXButton", "New session", "", 0, 1),                               // the header's
        n("AXButton", "New session", "", 0, 1),                               // the rail's foot
        n("AXButton", "New session: Kimi Code CLI · lab A", "", 0, 1),        // 3
        n("AXButton", "New session: Kimi Code CLI · lab B", "", 0, 1),
    ]
    if pick(rail, controls(rail, titled: "New session", roles: pressRoles), within: []).0 != nil { failures.append("new-session: the shared title chose one") }
    if pick(rail, controls(rail, titled: newSessionButton("Kimi Code CLI · lab A"), roles: pressRoles), within: []).0 != 3 { failures.append("new-session: the App's own action") }
    let parsed = parse(["Usage", "--region", "Main"])
    if parsed.region != "Main" || parsed.positional != ["Usage"] { failures.append("parse --region") }
    // Dev's main window, never another of its windows (dry run 3: a 280x168
    // one stood for it, so a control on the main window read "not inside
    // Dev's window", and the main window on another Space went unseen).
    let small = (title: "", frame: CGRect(x: 1200, y: 40, width: 280, height: 168), isMain: false)
    let floor = (title: "The Floor — 3 working", frame: CGRect(x: 0, y: 0, width: 1278, height: 798), isMain: true)
    let mainWin = (title: "AI Juggler · lab", frame: CGRect(x: 80, y: 60, width: 1360, height: 860), isMain: false)
    if pickMain([small, floor, mainWin]) != 2 { failures.append("main window: not the one titled AI Juggler") }
    if pickMain([small, floor]) != nil { failures.append("main window: another window stood for it") }
    if pickMain([(title: "AI Juggler", frame: CGRect(x: 0, y: 0, width: 1100, height: 700), isMain: false), mainWin]) != 1 { failures.append("main window: of two, the larger") }
    if pickMain([(title: "AI Juggler", frame: CGRect(x: 0, y: 0, width: 1100, height: 700), isMain: true), mainWin]) != 0 { failures.append("main window: of two, the main one") }
    if !isMainTitle("AI Juggler") || isMainTitle("AI Juggler Dev") || isMainTitle("Sign in · Kimi") { failures.append("main window: its title") }
    let cgs = [CGWin(id: 7, bounds: small.frame, title: "", onScreen: true),
               CGWin(id: 9, bounds: CGRect(x: 81, y: 61, width: 1359, height: 860), title: "AI Juggler · lab", onScreen: false)]
    if matchWindow(frame: mainWin.frame, title: mainWin.title, among: cgs)?.id != 9 { failures.append("main window: matched by frame") }
    if matchWindow(frame: CGRect(x: 500, y: 500, width: 1360, height: 860), title: mainWin.title, among: cgs) != nil { failures.append("main window: matched another frame") }
    let twins = cgs + [CGWin(id: 10, bounds: cgs[1].bounds, title: "AI Juggler · Home", onScreen: true)]
    if matchWindow(frame: mainWin.frame, title: "AI Juggler · lab", among: twins)?.id != 9 { failures.append("main window: two of one frame, by title") }
    if matchWindow(frame: mainWin.frame, title: "AI Juggler · Usage", among: twins) != nil { failures.append("main window: two of one frame, neither its title") }
    // Bringing Dev forward: a made-up clock and Mac.
    failures += activationSelftest()
    // Words: exact, squashed.
    if squash("  Sign\u{00a0}in \n") != "Sign in" { failures.append("squash") }
    if !shows(["OpenCode CLI · lab A needs its sign-in"], "lab A needs its sign-in") { failures.append("shows") }
    if shows(["Sign"], "Sign in") { failures.append("shows matched a prefix of the needle") }
    // Keys: the names the sheet uses exist.
    for k in ["return", "down", "up", "left", "right", "pagedown", "pageup", "home", "end", "escape", "tab", "space"] where keyCodes[k] == nil { failures.append("key \(k)") }
    // The wheel, a point and a size (F11).
    if wheelLines("down", nil) != -3 || wheelLines("up", "5") != 5 || wheelLines("left", "1") != nil || wheelLines("down", "0") != nil || wheelLines("down", "51") != nil { failures.append("scroll's lines") }
    if parsePoint("400, 300") != CGPoint(x: 400, y: 300) || parsePoint("400") != nil || parsePoint("a,b") != nil { failures.append("--at X,Y") }
    if parseSize("1024x680") != CGSize(width: 1024, height: 680) || parseSize("1280×800") != CGSize(width: 1280, height: 800) || parseSize("10x10") != nil || parseSize("1024") != nil { failures.append("resize WxH") }
    if parse(["down", "4", "--at", "10,20"]).at != "10,20" { failures.append("parse --at") }
    // Options parse.
    let o = parse(["Sign in", "--in", "lab A needs its sign-in", "--timeout", "900", "--ocr"])
    if o.positional != ["Sign in"] || o.within != ["lab A needs its sign-in"] || o.timeout != 900 || !o.ocr { failures.append("parse") }
    if failures.isEmpty { say("lab_drive: selftest passed (the guard's logic; nothing pressed or typed)"); exit(0) }
    failures.forEach { say("FAIL \($0)") }
    exit(1)
}

// MARK: - main

guard let command = args.first else { stop(2, "say a command (see the head of scripts/qa/lab_drive.swift)") }
if command == "--selftest" { selftest() }
let o = parse(Array(args.dropFirst()))
step = command
let pid = devPid()
ledgerFile = runFile(pid, ".input.json")
loadLedger()
// The run's log, in the run's own folder, appended to by each command.
do {
    let file = runFile(pid, ".log")
    let fm = FileManager.default
    let fresh = !fm.fileExists(atPath: file.path)
    try? fm.createDirectory(at: runDir, withIntermediateDirectories: true)
    if fresh { fm.createFile(atPath: file.path, contents: nil) }
    logFile = try? FileHandle(forWritingTo: file)
    _ = try? logFile?.seekToEnd()
    if logFile == nil { FileHandle.standardError.write("lab_drive: the run's log \(file.path) couldn't be opened; going on without it\n".data(using: .utf8)!) }
    if fresh { print("LOG \(file.path)") }
    record(runLine(args) + " (dev pid \(pid), driver pid \(getpid()))")
}
func need(_ n: Int, _ usage: String) { if o.positional.count < n { stop(2, usage) } }

switch command {
case "guard":
    guard let win = devWindow(pid) else { stop(3, describe(.offScreen)) }
    let front = NSWorkspace.shared.frontmostApplication
    let v = judge(dev: pid, frontmost: front?.processIdentifier, frontmostName: front?.localizedName ?? "",
                  windows: onScreenWindows(), point: CGPoint(x: win.bounds.midX, y: win.bounds.midY))
    say("\(describe(v)) — window \"\(win.title)\" \(Int(win.bounds.width))×\(Int(win.bounds.height))")
    exit(v == .clear ? 0 : 3)
case "texts": listTexts(pid, within: o.within, pressable: false)
case "buttons": listTexts(pid, within: o.within, pressable: true)
case "press": need(1, "press TITLE [--row NAME | --region NAME] [--in TEXT]"); press(pid, o.positional[0], within: o.within, row: o.row, region: o.region)
case "focus": need(1, "focus TITLE [--in TEXT]"); focusField(pid, o.positional[0], within: o.within)
case "focus-terminal": focusTerminal(pid, within: o.within)
case "type": need(1, "type TEXT"); say("» type \"\(o.positional.joined(separator: " "))\""); typeText(pid, o.positional.joined(separator: " "))
case "key":
    need(1, "key NAME [--repeat N]")
    say("» key \(o.positional[0])" + (o.repeatCount > 1 ? " ×\(o.repeatCount)" : ""))
    for _ in 0..<o.repeatCount { pressKey(pid, o.positional[0]) }
case "scroll":
    need(1, "scroll down|up [N] [--at X,Y]")
    guard let lines = wheelLines(o.positional[0], o.positional.count > 1 ? o.positional[1] : nil) else { stop(2, "scroll down|up [N, 1 to 50] [--at X,Y]") }
    let at = o.at.map { raw -> CGPoint in
        guard let point = parsePoint(raw) else { stop(2, "--at X,Y (screen points)") }
        return point
    }
    say("» scroll \(o.positional[0]) \(abs(lines))" + (o.at.map { " --at \($0)" } ?? ""))
    scrollWheel(pid, lines: lines, at: at)
case "resize":
    need(1, "resize WxH (Dev's main window, in points)")
    guard let size = parseSize(o.positional[0]) else { stop(2, "resize WxH, e.g. 1024x680") }
    say("» resize \(Int(size.width))×\(Int(size.height))")
    resizeMain(pid, to: size)
case "windows": listWindows(pid)
case "wait": need(1, "wait TEXT [--timeout S] [--ocr] [--gone]"); waitFor(pid, o.positional[0], timeout: o.timeout, ocr: o.ocr, gone: o.gone)
case "read": screenWords(pid, ocr: o.ocr).forEach { show($0) }
case "shot": shot(pid, o)
case "sign-in": need(1, "sign-in APP (its full name, e.g. \"OpenCode CLI · lab A\")"); signIn(pid, app: o.positional.joined(separator: " "))
case "read-label": need(1, "read-label APP"); readLabel(pid, app: o.positional[0])
case "new-session": need(3, "new-session TOOL APP PROJECT"); newSession(pid, tool: o.positional[0], app: o.positional[1], project: o.positional[2])
case "send": need(1, "send TEXT [--field TITLE]"); send(pid, o.positional.joined(separator: " "), field: o.field)
case "end-session": need(1, "end-session ROWTEXT [ROWTEXT …]"); endSession(pid, row: o.positional + o.within)
case "resume": need(1, "resume ROWTEXT [ROWTEXT …]"); press(pid, "Resume", within: o.positional + o.within)
case "open-usage":
    need(1, "open-usage APP")
    // The header's Usage screen, by its navigation's name: a page's own
    // tab is another control (AJ-E1612 dry run 3: two titled "Usage").
    press(pid, "Usage", within: [], region: "Main")
    waitFor(pid, o.positional[0], timeout: 15, ocr: false, gone: false)
    listTexts(pid, within: [o.positional[0]], pressable: false)
case "lab-b-session":
    need(3, "lab-b-session TOOL APP PROJECT")
    newSession(pid, tool: o.positional[0], app: o.positional[1], project: o.positional[2], labB: true)
    usleep(1_500_000)
    say("what Dev says now (lab B must be refused or asked to sign in, never run on lab A's account):")
    listTexts(pid, within: ["New session"], pressable: false)
default: stop(2, "unknown command \(command)")
}
record("EXIT 0")
