// AJ-E1612: the sign-in sitting's guarded driver for AI Juggler Dev.
//
// Drives AI Juggler Dev by its pid only, never by name (the installed AI
// Juggler has the same process name, juggler-app), and refuses any pid
// that isn't an "AI Juggler Dev.app". Never raises or activates any app,
// never moves the pointer: buttons are pressed through accessibility,
// keys are posted to Dev's pid only.
//
// THE CLICK GUARD. Before every press and every keystroke, both must hold:
//   - the frontmost app (NSWorkspace) is Dev's pid;
//   - the topmost on-screen window at the point (CGWindowList, active
//     Space, front to back) belongs to Dev's pid.
// If either fails, nothing is sent: the driver prints what is in the way
// and waits (AIJ_DRIVE_WAIT seconds, default 120) for the person at the
// Mac to bring Dev forward; still failing, it stops (exit 3), nothing sent.
//
// Build once (system swiftc only), then run:
//   swiftc -O -o "$S/lab_drive" scripts/qa/lab_drive.swift      # $S: a scratch folder
//   AIJ_DEV_PID=<pid> AIJ_DRIVE_OUT=docs/qa/evidence/AJ-E1612 "$S/lab_drive" <command> …
//   "$S/lab_drive" --selftest                                     # the guard's logic, no clicks
//
// Primitives (TITLE and TEXT are exact words; --in TEXT scopes to the
// smallest part of the window holding both TEXT and the control):
//   guard                         the guard's verdict at Dev's window, nothing sent
//   texts [--in TEXT]             the words Dev shows (role, label, value), read only
//   buttons [--in TEXT]           the controls that can be pressed, with their exact titles
//   press TITLE [--in TEXT …]     presses the one control titled exactly TITLE (each --in
//                                 adds words its region must hold)
//   focus TITLE [--in TEXT]       puts the cursor in the field titled TITLE
//   focus-terminal [--in TEXT]    puts the cursor in AI Juggler's terminal (its input)
//   type TEXT                     types TEXT where the cursor is (keys posted to Dev's pid)
//   key NAME [--repeat N]         return, down, up, left, right, escape, tab, space, ctrl-c
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
//   sign-in ROWTEXT [ROWTEXT …]   Home's Needs-you row holding every ROWTEXT (e.g. "lab A
//                                 needs its sign-in" "OpenCode's own sign-in"): its Sign in;
//                                 waits for the sign-in terminal ("Sign in · …")
//   read-label APP                the sign-in words beside APP (Signed in / Not signed in …)
//   new-session TOOL APP PROJECT  New session → Change app → APP → Change folder →
//                                 PROJECT → "Start in PROJECT" (TOOL names the row in the log)
//   send TEXT [--field TITLE]     types TEXT and Return into the session's terminal
//                                 (or, for a session without one, the field TITLE)
//   end-session ROWTEXT           "Stop this session" in the row holding ROWTEXT
//   resume ROWTEXT                "Resume" in the row holding ROWTEXT
//   open-usage APP                the Usage screen, then the words beside APP
//   lab-b-session TOOL APP PROJECT
//                                 new-session on lab B; prints what Dev says instead
//
// Exit: 0 done, 2 usage, 3 the guard stopped it (nothing sent), 4 not
// found / ambiguous (nothing sent), 5 timed out waiting.
import AppKit
import ApplicationServices
import CoreGraphics
import Foundation
import Vision

// MARK: - plain helpers

func say(_ text: String) { print(text); fflush(stdout) }

func stop(_ code: Int32, _ message: String) -> Never {
    FileHandle.standardError.write("lab_drive: \(message)\n".data(using: .utf8)!)
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
    }
}

// MARK: - Dev, by pid only

let env = ProcessInfo.processInfo.environment
let args = Array(CommandLine.arguments.dropFirst())
let outDir = URL(fileURLWithPath: env["AIJ_DRIVE_OUT"] ?? FileManager.default.currentDirectoryPath)
let waitLimit = Double(env["AIJ_DRIVE_WAIT"] ?? "") ?? 120

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

/// Dev's front window (layer 0) on screen: its id and bounds.
func devWindow(_ pid: pid_t) -> (id: CGWindowID, bounds: CGRect, title: String)? {
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
    for w in list where (w[kCGWindowOwnerPID as String] as? pid_t) == pid && (w[kCGWindowLayer as String] as? Int) == 0 {
        guard let id = w[kCGWindowNumber as String] as? CGWindowID,
              let raw = w[kCGWindowBounds as String] as? NSDictionary,
              let bounds = CGRect(dictionaryRepresentation: raw), bounds.width > 50, bounds.height > 50 else { continue }
        return (id, bounds, w[kCGWindowName as String] as? String ?? "")
    }
    return nil
}

/// The guard, right before a press or a key. Waits (asking once) while it
/// fails; stops with nothing sent if it still fails after AIJ_DRIVE_WAIT.
func guarded(_ pid: pid_t, at point: CGPoint) {
    let start = Date()
    var asked = ""
    while true {
        let front = NSWorkspace.shared.frontmostApplication
        let verdict = judge(dev: pid, frontmost: front?.processIdentifier, frontmostName: front?.localizedName ?? "",
                            windows: onScreenWindows(), point: point)
        if verdict == .clear { return }
        let line = describe(verdict)
        if line != asked {
            say("WAITING: \(line). Nothing is sent. Please bring AI Juggler Dev forward (click its window); "
                + "the driver never does it itself. Waiting up to \(Int(waitLimit)) s.")
            asked = line
        }
        if Date().timeIntervalSince(start) > waitLimit { stop(3, "stopped by the guard: \(line). Nothing was sent.") }
        usleep(500_000)
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

/// Of `candidates`, the one in the tightest region holding every one of
/// `regions`' words: for each candidate, its deepest ancestor that also
/// holds a node with each text. Ambiguous or none: nil, with the reason.
func pick(_ nodes: [Node], _ candidates: [Int], within regions: [String]) -> (Int?, String) {
    guard !regions.isEmpty else {
        if candidates.count == 1 { return (candidates[0], "") }
        return (nil, candidates.isEmpty ? "none found" : "\(candidates.count) found: say which with --in TEXT")
    }
    var markUps: [[Set<Int>]] = []  // per region text: each mark's ancestors
    for region in regions {
        let marks = nodes.indices.filter { nodes[$0].words.contains(region) }
        if marks.isEmpty { return (nil, "no words \"\(region)\" on screen") }
        markUps.append(marks.map { Set(ancestors(nodes, $0)) })
    }
    var best: [(Int, Int)] = []  // (candidate, its region's depth)
    for c in candidates {
        // The first ancestor (from the control up) holding every text.
        if let region = ancestors(nodes, c).first(where: { a in markUps.allSatisfy { ups in ups.contains { $0.contains(a) } } }) {
            best.append((c, nodes[region].depth))
        }
    }
    let named = regions.map { "\"\($0)\"" }.joined(separator: " + ")
    guard let top = best.map({ $0.1 }).max() else { return (nil, "none found in a region with \(named)") }
    let winners = best.filter { $0.1 == top }.map { $0.0 }
    if winners.count == 1 { return (winners[0], "") }
    return (nil, "\(winners.count) found equally near \(named): add another --in")
}

func center(_ e: AXUIElement) -> CGPoint? {
    guard let f = frame(e), f.width > 0, f.height > 0 else { return nil }
    return CGPoint(x: f.midX, y: f.midY)
}

/// A point inside Dev's front window for the element (scrolled into view first).
func pointFor(_ pid: pid_t, _ e: AXUIElement) -> CGPoint {
    guard let win = devWindow(pid) else { stop(3, "AI Juggler Dev has no window on screen; nothing sent") }
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
                                     "left": 123, "right": 124, "down": 125, "up": 126, "delete": 51, "c": 8]

func keyPoint(_ pid: pid_t) -> CGPoint {
    let app = AXUIElementCreateApplication(pid)
    if let focused = attr(app, kAXFocusedUIElementAttribute), let c = center(focused as! AXUIElement),
       let win = devWindow(pid), win.bounds.contains(c) { return c }
    guard let win = devWindow(pid) else { stop(3, "AI Juggler Dev has no window on screen; nothing sent") }
    return CGPoint(x: win.bounds.midX, y: win.bounds.midY)
}

func post(_ pid: pid_t, code: CGKeyCode, flags: CGEventFlags = [], unicode: String? = nil) {
    guarded(pid, at: keyPoint(pid))
    for down in [true, false] {
        guard let event = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: down) else { stop(3, "no key event") }
        event.flags = flags
        if let unicode {
            let units = Array(unicode.utf16)
            event.keyboardSetUnicodeString(stringLength: units.count, unicodeString: units)
        }
        event.postToPid(pid)
        usleep(12_000)
    }
}

func typeText(_ pid: pid_t, _ s: String) {
    for ch in s { post(pid, code: 0, unicode: String(ch)) }
}

func pressKey(_ pid: pid_t, _ name: String) {
    if name == "ctrl-c" { post(pid, code: 8, flags: .maskControl); return }
    guard let code = keyCodes[name] else { stop(2, "unknown key \(name) (return, down, up, left, right, escape, tab, space, ctrl-c)") }
    post(pid, code: code)
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
    var timeout: Double = 30
    var ocr = false
    var gone = false
    var repeatCount = 1
    var field: String?
    var boxes: [String] = []
    var noRealData: String?
    var note: String?
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
        case "--timeout": o.timeout = Double(value()) ?? 30
        case "--ocr": o.ocr = true
        case "--gone": o.gone = true
        case "--repeat": o.repeatCount = max(1, Int(value()) ?? 1)
        case "--field": o.field = value()
        case "--box": o.boxes.append(value())
        case "--no-real-data": o.noRealData = value()
        case "--note": o.note = value()
        default: o.positional.append(raw[i])
        }
        i += 1
    }
    return o
}

/// Presses it; with `soft`, a control that is there but disabled is
/// reported (false) instead of stopping.
@discardableResult
func press(_ pid: pid_t, _ title: String, within: [String], soft: Bool = false) -> Bool {
    say("» press \"\(title)\"" + within.map { " --in \"\($0)\"" }.joined())
    let nodes = tree(pid)
    let (chosen, why) = pick(nodes, controls(nodes, titled: title, roles: pressRoles), within: within)
    guard let i = chosen else { stop(4, "no single control titled exactly \"\(title)\": \(why). Nothing pressed. (`buttons` lists them.)") }
    let e = nodes[i].element
    if soft, (attr(e, kAXEnabledAttribute) as? Bool) == false {
        say("  \"\(title)\" is there but disabled: not pressed")
        return false
    }
    let point = pointFor(pid, e)
    guarded(pid, at: point)
    let result = AXUIElementPerformAction(e, kAXPressAction as CFString)
    guard result == .success else { stop(4, "\"\(title)\" didn't take the press (\(result.rawValue))") }
    usleep(300_000)
    return true
}

func focusField(_ pid: pid_t, _ title: String, within: [String]) {
    say("» focus \"\(title)\"" + within.map { " --in \"\($0)\"" }.joined())
    let nodes = tree(pid)
    let (chosen, why) = pick(nodes, controls(nodes, titled: title, roles: fieldRoles), within: within)
    guard let i = chosen else { stop(4, "no single field titled exactly \"\(title)\": \(why)") }
    guarded(pid, at: pointFor(pid, nodes[i].element))
    AXUIElementSetAttributeValue(nodes[i].element, kAXFocusedAttribute as CFString, kCFBooleanTrue)
    usleep(200_000)
}

/// AI Juggler's terminal input: xterm's text area, the one whose
/// description names the terminal, else the only text area in the region.
func focusTerminal(_ pid: pid_t, within: [String]) {
    say("» focus-terminal" + within.map { " --in \"\($0)\"" }.joined())
    let nodes = tree(pid)
    let areas = nodes.indices.filter { nodes[$0].role == "AXTextArea" && (nodes[$0].label.lowercased().contains("terminal")) }
    let fallback = nodes.indices.filter { nodes[$0].role == "AXTextArea" }
    var (chosen, why) = pick(nodes, areas, within: within)
    if chosen == nil, areas.isEmpty { (chosen, why) = pick(nodes, fallback, within: within) }
    guard let i = chosen else { stop(4, "no single terminal input: \(why)") }
    guarded(pid, at: pointFor(pid, nodes[i].element))
    AXUIElementSetAttributeValue(nodes[i].element, kAXFocusedAttribute as CFString, kCFBooleanTrue)
    usleep(200_000)
}

func waitFor(_ pid: pid_t, _ needle: String, timeout: Double, ocr: Bool, gone: Bool) {
    say("» wait \"\(needle)\"" + (gone ? " --gone" : "") + (ocr ? " --ocr" : "") + " --timeout \(Int(timeout))")
    let start = Date()
    var lastOcr = Date.distantPast
    while true {
        let useOcr = ocr && Date().timeIntervalSince(lastOcr) > 1.5
        if useOcr { lastOcr = Date() }
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
            say("\(n.role)\t\"\(title)\"")
        } else if !n.words.isEmpty || !n.value.isEmpty {
            say("\(String(repeating: " ", count: min(n.depth, 30)))\(n.role)\t\(n.label)\t\(n.value)")
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

func signIn(_ pid: pid_t, row: [String]) {
    press(pid, "Sign in", within: row)
    waitFor(pid, "Sign in ·", timeout: 20, ocr: false, gone: false)
    say("the sign-in terminal is open: answer its menu with `key`/`type`, read it with `read --ocr`")
}

let signInWords = ["Signed in", "Not signed in", "needs its sign-in", "Account unknown", "Sign in"]

func readLabel(_ pid: pid_t, app: String) {
    let nodes = tree(pid)
    let marks = nodes.indices.filter { nodes[$0].words.contains(app) }
    guard !marks.isEmpty else { stop(4, "no words \"\(app)\" on screen") }
    var seen: [String] = []
    for m in marks {
        // The row: up to four levels above the App's name.
        let row = ancestors(nodes, m).prefix(5).last!
        for j in [row] + descendants(nodes, row) {
            for w in signInWords where nodes[j].words.contains(w) && !seen.contains(nodes[j].words) { seen.append(nodes[j].words) }
        }
    }
    say(seen.isEmpty ? "no sign-in words beside \"\(app)\"" : seen.map { "\(app): \($0)" }.joined(separator: "\n"))
}

func newSession(_ pid: pid_t, tool: String, app: String, project: String, labB: Bool = false) {
    say("new session: \(tool) on \(app) in \(project)")
    press(pid, "New session", within: [])
    waitFor(pid, "Change app", timeout: 10, ocr: false, gone: false)
    press(pid, "Change app", within: ["New session"])
    waitFor(pid, app, timeout: 10, ocr: false, gone: false)
    press(pid, app, within: ["Which app"])
    press(pid, "Change folder", within: ["New session"])
    waitFor(pid, "Where to work", timeout: 10, ocr: false, gone: false)
    press(pid, project, within: ["Where to work"])
    let started = press(pid, "Start in \(project)", within: ["New session"], soft: labB)
    if labB { say(started ? "  Start was pressed: see what Dev shows next" : "  Dev didn't offer Start on lab B") }
}

func send(_ pid: pid_t, _ words: String, field: String?) {
    if let field { focusField(pid, field, within: []) } else { focusTerminal(pid, within: []) }
    say("» type \"\(words)\" + return")
    typeText(pid, words)
    pressKey(pid, "return")
}

// MARK: - the selftest (no clicks, no keys)

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
    if pick(rows, signIns, within: ["lab A needs its sign-in", "OpenCode's own sign-in"]).0 != 4 { failures.append("pick OpenCode's row") }
    if pick(rows, signIns, within: ["Kimi Code's own sign-in"]).0 != 9 { failures.append("pick Kimi's row") }
    if pick(rows, signIns, within: ["lab A needs its sign-in"]).0 != nil { failures.append("an ambiguous region picked one") }
    if pick(rows, signIns, within: []).0 != nil { failures.append("two Sign in without a region picked one") }
    if pick(rows, signIns, within: ["Grok Build's own sign-in"]).0 != nil { failures.append("a missing region picked one") }
    if controls(rows, titled: "Sign", roles: pressRoles) != [] { failures.append("a prefix matched") }
    // Words: exact, squashed.
    if squash("  Sign\u{00a0}in \n") != "Sign in" { failures.append("squash") }
    if !shows(["OpenCode CLI · lab A needs its sign-in"], "lab A needs its sign-in") { failures.append("shows") }
    if shows(["Sign"], "Sign in") { failures.append("shows matched a prefix of the needle") }
    // Keys: the names the sheet uses exist.
    for k in ["return", "down", "up", "escape", "tab", "space"] where keyCodes[k] == nil { failures.append("key \(k)") }
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
let pid = devPid()
func need(_ n: Int, _ usage: String) { if o.positional.count < n { stop(2, usage) } }

switch command {
case "guard":
    guard let win = devWindow(pid) else { stop(3, "AI Juggler Dev has no window on screen") }
    let front = NSWorkspace.shared.frontmostApplication
    let v = judge(dev: pid, frontmost: front?.processIdentifier, frontmostName: front?.localizedName ?? "",
                  windows: onScreenWindows(), point: CGPoint(x: win.bounds.midX, y: win.bounds.midY))
    say("\(describe(v)) — window \"\(win.title)\" \(Int(win.bounds.width))×\(Int(win.bounds.height))")
    exit(v == .clear ? 0 : 3)
case "texts": listTexts(pid, within: o.within, pressable: false)
case "buttons": listTexts(pid, within: o.within, pressable: true)
case "press": need(1, "press TITLE [--in TEXT]"); press(pid, o.positional[0], within: o.within)
case "focus": need(1, "focus TITLE [--in TEXT]"); focusField(pid, o.positional[0], within: o.within)
case "focus-terminal": focusTerminal(pid, within: o.within)
case "type": need(1, "type TEXT"); say("» type \"\(o.positional.joined(separator: " "))\""); typeText(pid, o.positional.joined(separator: " "))
case "key":
    need(1, "key NAME [--repeat N]")
    say("» key \(o.positional[0])" + (o.repeatCount > 1 ? " ×\(o.repeatCount)" : ""))
    for _ in 0..<o.repeatCount { pressKey(pid, o.positional[0]) }
case "wait": need(1, "wait TEXT [--timeout S] [--ocr] [--gone]"); waitFor(pid, o.positional[0], timeout: o.timeout, ocr: o.ocr, gone: o.gone)
case "read": screenWords(pid, ocr: o.ocr).forEach { say($0) }
case "shot": shot(pid, o)
case "sign-in": need(1, "sign-in ROWTEXT [ROWTEXT …]"); signIn(pid, row: o.positional + o.within)
case "read-label": need(1, "read-label APP"); readLabel(pid, app: o.positional[0])
case "new-session": need(3, "new-session TOOL APP PROJECT"); newSession(pid, tool: o.positional[0], app: o.positional[1], project: o.positional[2])
case "send": need(1, "send TEXT [--field TITLE]"); send(pid, o.positional.joined(separator: " "), field: o.field)
case "end-session": need(1, "end-session ROWTEXT [ROWTEXT …]"); press(pid, "Stop this session", within: o.positional + o.within)
case "resume": need(1, "resume ROWTEXT [ROWTEXT …]"); press(pid, "Resume", within: o.positional + o.within)
case "open-usage":
    need(1, "open-usage APP")
    press(pid, "Usage", within: [])
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
