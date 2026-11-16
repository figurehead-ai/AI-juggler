// AJ-E3013 §7.2: the Claude app driver for AI Juggler's lab proofs. It
// operates ONE Claude desktop app window, by process id, on the sanctioned
// test profile (aij-test). Modelled on floor_drive.swift; how to use it:
// docs/qa/claude-app-drive.md.
//
//   AIJ_APP_PID=<pid> AIJ_APP_PROFILE=aij-test AIJ_APP_OUT=<folder> \
//     scripts/qa/app_drive <verb> …
//
//   list                 the running Claude app processes: pid, whether it
//                        is AIJ_APP_PROFILE's (no other profile is named),
//                        its main window (needs no AIJ_APP_PID)
//   shot                 a picture of the pid's main window at its size in
//                        points (a pixel is a click's point); prints its
//                        path and the size
//   click X Y            presses what is at (X, Y) from the window's top
//                        left through accessibility (AXPress), or puts the
//                        cursor in the text field there
//   type TEXT            types TEXT into the field with the cursor: key
//                        events posted to the pid only
//   key NAME             return, escape, tab, space, delete, up, down,
//                        left, right, home, end, pageup, pagedown, or a
//                        combination like cmd+n or shift+tab (to the pid)
//   menu                 lists the menu bar; menu "File" lists a menu
//   menu "File>Item"     presses that item (exact titles, never a prefix)
//   front                brings the window forward (the Mac's keyboard
//                        focus moves to it): only where a step needs it
//   --selftest           the guard's refusals, without a Claude app
//
// The verbs that act answer with the time until the window changed (up to
// 3 s).
//
// THE GUARD, read at that moment before every verb on a pid:
//   1. AIJ_APP_PID is a running process whose executable is the main one of
//      a bundle named exactly Claude.app with the Claude app's bundle id:
//      never AI Juggler, a terminal, the Claude Code CLI (a lowercase
//      claude.app) or any other app;
//   2. AIJ_APP_PROFILE is set, and the process's --user-data-dir ends with
//      that profile folder (the default profile, started without one, is
//      refused);
//   3. its main window exists and is on screen (for shot, click, type, key
//      and a menu press; `front` is how a window off screen comes back).
// Before each key event, rule 1 is read again. Any refusal prints its
// reason and exits 2.
//
// Never: a global mouse or key event, moving the pointer, a keystroke to
// another process, reading the app's profile files, any credential,
// Keychain item or token. The process's argv is read for --user-data-dir
// only; the environment that follows it in the same kernel buffer is wiped
// unread.

import AppKit
import ApplicationServices
import Carbon
import CoreGraphics
import Darwin
import Foundation

let claudeBundleName = "Claude.app"
let claudeBundleID = "com.anthropic.claudefordesktop"

func fail(_ message: String) -> Never {
    FileHandle.standardError.write("app_drive: \(message)\n".data(using: .utf8)!)
    exit(2)
}

let env = ProcessInfo.processInfo.environment
let out = URL(fileURLWithPath: env["AIJ_APP_OUT"] ?? ((NSTemporaryDirectory() as NSString).appendingPathComponent("aij-app-drive/out")))

// MARK: the guard's rule 1: a Claude app process

/// The executable of `pid`, or nil when there is no such process.
func executablePath(_ pid: pid_t) -> String? {
    guard pid > 0 else { return nil }
    var buf = [CChar](repeating: 0, count: 4 * Int(MAXPATHLEN))
    let n = proc_pidpath(pid, &buf, UInt32(buf.count))
    guard n > 0 else { return nil }
    return String(decoding: buf.prefix(Int(n)).map { UInt8(bitPattern: $0) }, as: UTF8.self)
}

/// Rule 1 on a path alone: the main executable of a bundle named exactly
/// Claude.app (…/Claude.app/Contents/MacOS/<exe>). Case matters: the
/// Claude Code CLI ships as a lowercase claude.app.
func claudeExecutableRefusal(_ path: String) -> String? {
    let parts = (path as NSString).standardizingPath.split(separator: "/").map(String.init)
    let n = parts.count
    if n >= 4, parts[n - 2] == "MacOS", parts[n - 3] == "Contents", parts[n - 4] == claudeBundleName {
        return nil
    }
    let bundle = parts.last(where: { $0.hasSuffix(".app") }).map { " (in \($0))" } ?? ""
    return "runs \(parts.last ?? "?")\(bundle), not the Claude app's own executable (\(claudeBundleName)/Contents/MacOS)"
}

/// Rule 1: `pid` is a running Claude app (path, bundle name and bundle id).
func identityRefusal(_ pid: pid_t) -> String? {
    guard pid > 1 else { return "pid \(pid) isn't an app's" }
    if kill(pid, 0) != 0 && errno == ESRCH { return "no process has pid \(pid)" }
    guard let exe = executablePath(pid) else { return "no process has pid \(pid) (or its executable can't be read)" }
    if let refusal = claudeExecutableRefusal(exe) { return "pid \(pid) \(refusal)" }
    guard let app = NSRunningApplication(processIdentifier: pid), !app.isTerminated else {
        return "pid \(pid) isn't a running app"
    }
    guard app.bundleIdentifier == claudeBundleID else {
        return "pid \(pid)'s bundle id isn't the Claude app's (\(claudeBundleID))"
    }
    guard let bundle = app.bundleURL?.resolvingSymlinksInPath().path,
          (bundle as NSString).lastPathComponent == claudeBundleName,
          URL(fileURLWithPath: exe).resolvingSymlinksInPath().path.hasPrefix(bundle + "/Contents/MacOS/") else {
        return "pid \(pid)'s executable isn't its Claude.app bundle's own"
    }
    return nil
}

// MARK: the guard's rule 2: the profile

/// The process's argv (argv[0] first), from the kernel's copy. The buffer
/// also holds the environment after argv: it's wiped, never parsed.
func arguments(_ pid: pid_t) -> [String]? {
    var mib: [Int32] = [CTL_KERN, KERN_ARGMAX]
    var argmax: Int32 = 0
    var size = MemoryLayout<Int32>.size
    guard sysctl(&mib, 2, &argmax, &size, nil, 0) == 0, argmax > 0 else { return nil }
    var buf = [UInt8](repeating: 0, count: Int(argmax))
    defer { _ = buf.withUnsafeMutableBytes { memset_s($0.baseAddress, $0.count, 0, $0.count) } }
    mib = [CTL_KERN, KERN_PROCARGS2, pid]
    size = buf.count
    guard sysctl(&mib, 3, &buf, &size, nil, 0) == 0, size > MemoryLayout<Int32>.size else { return nil }
    let argc = Int(buf.withUnsafeBytes { $0.load(as: Int32.self) })
    var i = MemoryLayout<Int32>.size
    while i < size && buf[i] != 0 { i += 1 }  // the executable's path
    while i < size && buf[i] == 0 { i += 1 }  // its padding
    var args = [String]()
    while args.count < argc && i < size {
        let start = i
        while i < size && buf[i] != 0 { i += 1 }
        args.append(String(decoding: buf[start..<i], as: UTF8.self))
        i += 1
    }
    return args.count == argc ? args : nil
}

/// The --user-data-dir values the app reads from argv. Chromium takes a
/// switch as "--name=value" (or "-name=value") up to a "--"; a bare
/// "--user-data-dir" with its value in the next argument isn't read as
/// one, so it's flagged and refused rather than guessed.
func userDataDirs(_ argv: [String]) -> (values: [String], bare: Bool) {
    var values = [String](), bare = false
    let name = "user-data-dir"
    for a in argv.dropFirst() {
        if a == "--" { break }
        for prefix in ["--", "-"] where a.hasPrefix(prefix + name) {
            let rest = a.dropFirst(prefix.count + name.count)
            if rest.isEmpty { bare = true } else if rest.first == "=" { values.append(String(rest.dropFirst())) }
            break
        }
    }
    return (values, bare)
}

/// Rule 2: AIJ_APP_PROFILE is set and the app's one --user-data-dir ends
/// with it, by whole folder names ("aij-test", or "ClaudeProfiles/aij-test").
func profileRefusal(argv: [String]?, profile: String?) -> String? {
    guard let profile, !profile.isEmpty else {
        return "set AIJ_APP_PROFILE to the test profile's folder name (aij-test): the driver acts only on that profile's app"
    }
    let want = profile.split(separator: "/").map(String.init)
    guard !want.isEmpty, !want.contains("."), !want.contains("..") else {
        return "AIJ_APP_PROFILE must name a profile folder (like aij-test)"
    }
    guard let argv else { return "its arguments can't be read, so its profile can't be checked" }
    let (values, bare) = userDataDirs(argv)
    if bare { return "it was started with a --user-data-dir that has no value" }
    guard values.count == 1 else {
        return values.isEmpty ? "it runs the default profile (no --user-data-dir), not \(profile)"
                              : "it was started with more than one --user-data-dir"
    }
    let have = (values[0] as NSString).standardizingPath.split(separator: "/").map(String.init)
    guard have.count >= want.count, Array(have.suffix(want.count)) == want else {
        return "its --user-data-dir isn't the \(profile) profile's folder"
    }
    return nil
}

// MARK: windows and accessibility

struct Win { let id: CGWindowID; let bounds: CGRect }

func attr(_ e: AXUIElement, _ name: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(e, name as CFString, &value) == .success ? value : nil
}

func element(_ v: CFTypeRef?) -> AXUIElement? {
    guard let v, CFGetTypeID(v) == AXUIElementGetTypeID() else { return nil }
    return (v as! AXUIElement)
}

func children(_ e: AXUIElement) -> [AXUIElement] {
    (attr(e, kAXChildrenAttribute) as? [AXUIElement]) ?? []
}

func text(_ e: AXUIElement, _ name: String) -> String { (attr(e, name) as? String) ?? "" }

func frame(_ e: AXUIElement) -> CGRect? {
    guard let pv = attr(e, kAXPositionAttribute), let sv = attr(e, kAXSizeAttribute),
          CFGetTypeID(pv) == AXValueGetTypeID(), CFGetTypeID(sv) == AXValueGetTypeID() else { return nil }
    var p = CGPoint.zero, s = CGSize.zero
    guard AXValueGetValue(pv as! AXValue, .cgPoint, &p), AXValueGetValue(sv as! AXValue, .cgSize, &s) else { return nil }
    return CGRect(origin: p, size: s)
}

func near(_ a: CGRect, _ b: CGRect) -> Bool {
    abs(a.minX - b.minX) <= 2 && abs(a.minY - b.minY) <= 2 && abs(a.width - b.width) <= 2 && abs(a.height - b.height) <= 2
}

func actions(_ e: AXUIElement) -> [String] {
    var names: CFArray?
    AXUIElementCopyActionNames(e, &names)
    return (names as? [String]) ?? []
}

func ancestors(_ start: AXUIElement) -> [AXUIElement] {
    var all = [start]
    while all.count < 12, let parent = element(attr(all[all.count - 1], kAXParentAttribute)) { all.append(parent) }
    return all
}

/// The pid's ordinary windows on screen (layer 0, shown, not a sliver).
func windowsOnScreen(_ pid: pid_t) -> [Win] {
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
    return list.compactMap { w in
        guard (w[kCGWindowOwnerPID as String] as? pid_t) == pid, (w[kCGWindowLayer as String] as? Int) == 0,
              ((w[kCGWindowAlpha as String] as? Double) ?? 1) > 0,
              let id = w[kCGWindowNumber as String] as? CGWindowID,
              let raw = w[kCGWindowBounds as String] as? NSDictionary,
              let b = CGRect(dictionaryRepresentation: raw), b.width >= 200, b.height >= 150 else { return nil }
        return Win(id: id, bounds: b)
    }
}

/// The app being driven, once rules 1 and 2 passed.
struct Driven {
    let pid: pid_t
    let exe: String
    let ax: AXUIElement

    /// Its main window: the one accessibility calls main, else the largest.
    func mainWindow() -> Win? {
        let wins = windowsOnScreen(pid)
        if wins.count <= 1 { return wins.first }
        if let m = element(attr(ax, kAXMainWindowAttribute)), let f = frame(m), let w = wins.first(where: { near($0.bounds, f) }) {
            return w
        }
        return wins.max { $0.bounds.width * $0.bounds.height < $1.bounds.width * $1.bounds.height }
    }

    /// Rule 3.
    func needWindow() -> Win {
        guard let w = mainWindow() else {
            fail("pid \(pid) has no window on screen (on another Space, minimised or closed; `front` brings it forward)")
        }
        return w
    }

    func axWindow(_ w: Win) -> AXUIElement? {
        ((attr(ax, kAXWindowsAttribute) as? [AXUIElement]) ?? []).first { frame($0).map { near($0, w.bounds) } ?? false }
    }

    /// Chromium builds its web content's tree for accessibility only when
    /// asked: this asks (an Electron app's documented switch).
    func enableWebTree() {
        AXUIElementSetAttributeValue(ax, "AXManualAccessibility" as CFString, kCFBooleanTrue)
    }
}

func pidOf(_ e: AXUIElement) -> pid_t {
    var p: pid_t = 0
    AXUIElementGetPid(e, &p)
    return p
}

/// Rules 1 and 2 for `AIJ_APP_PID`, then accessibility access if needed.
func guardProcess(needsAccessibility: Bool) -> Driven {
    guard let pidText = env["AIJ_APP_PID"], !pidText.isEmpty else { fail("set AIJ_APP_PID to the Claude app's pid (`list` shows them)") }
    guard pidText.allSatisfy(\.isASCII), let pid = pid_t(pidText) else { fail("AIJ_APP_PID isn't a pid: \(pidText)") }
    if let refusal = identityRefusal(pid) { fail(refusal) }
    if let refusal = profileRefusal(argv: arguments(pid), profile: env["AIJ_APP_PROFILE"]) { fail("pid \(pid): \(refusal)") }
    if needsAccessibility && !AXIsProcessTrusted() {
        fail("this terminal has no accessibility access (granting it is the maintainer's step)")
    }
    return Driven(pid: pid, exe: executablePath(pid) ?? "", ax: AXUIElementCreateApplication(pid))
}

// MARK: pictures

func capture(_ w: Win, to file: URL) -> Bool {
    try? FileManager.default.createDirectory(at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
    try? FileManager.default.removeItem(at: file)  // screencapture won't overwrite
    let task = Process()
    task.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
    task.arguments = ["-x", "-o", "-l\(w.id)", file.path]
    do { try task.run() } catch { return false }
    task.waitUntilExit()
    return task.terminationStatus == 0 && FileManager.default.fileExists(atPath: file.path)
}

let probeFile = out.appendingPathComponent(".app_drive-changed.png")

func picture(_ w: Win) -> Data {
    capture(w, to: probeFile) ? ((try? Data(contentsOf: probeFile)) ?? Data()) : Data()
}

/// Waits until the window's picture changes (up to 3 s).
func changed(_ d: Driven, _ w: Win, from before: Data, since start: Date) -> String {
    defer { try? FileManager.default.removeItem(at: probeFile) }
    while Date().timeIntervalSince(start) < 3 {
        usleep(30_000)
        let ms = Int(Date().timeIntervalSince(start) * 1000)
        guard windowsOnScreen(d.pid).contains(where: { $0.id == w.id }) else { return "the window went away in \(ms) ms" }
        let now = picture(w)
        if !now.isEmpty && now != before { return "changed in \(Int(Date().timeIntervalSince(start) * 1000)) ms" }
    }
    return before.isEmpty ? "done; not timed (the window couldn't be captured)" : "no change in 3 s"
}

// MARK: clicks

let textRoles: Set<String> = ["AXTextField", "AXTextArea", "AXComboBox", "AXSearchField"]
func isTextEntry(_ e: AXUIElement) -> Bool { textRoles.contains(text(e, kAXRoleAttribute)) }

/// Roles never pressed as "what's there": pressing them clicks a whole page.
let wholeRoles: Set<String> = ["AXWebArea", "AXScrollArea", "AXWindow", "AXApplication", "AXSplitGroup"]
let windowControls: Set<String> = ["AXCloseButton", "AXMinimizeButton", "AXZoomButton", "AXFullScreenButton"]

/// Presses that would sign the test profile out or quit it are refused.
func pressRefusal(_ label: String) -> String? {
    let l = label.lowercased()
    for p in ["log out", "logout", "log off", "sign out", "quit", "delete account"] where l.hasPrefix(p) {
        return "\"\(label)\" signs out, quits or deletes: the driver doesn't press it"
    }
    return nil
}

/// The deepest element of the window under the point, found in its own
/// tree (the fallback when the app's hit test answers from elsewhere).
func walk(_ root: AXUIElement, _ at: CGPoint) -> AXUIElement? {
    var best: (AXUIElement, Int)?
    func visit(_ e: AXUIElement, _ depth: Int) {
        if depth > 60 { return }
        guard let f = frame(e), f.contains(at) else {
            if depth < 3 { children(e).forEach { visit($0, depth + 1) } }  // a frameless container may hold it
            return
        }
        if best.map({ depth >= $0.1 }) ?? true { best = (e, depth) }
        children(e).forEach { visit($0, depth + 1) }
    }
    visit(root, 0)
    return best?.0
}

/// What's at the point in that window: the app's own hit test (asked of
/// the pid, so another app's window over the point doesn't count), else
/// the window's tree. The web content's tree may take a moment to build.
func elementAt(_ d: Driven, _ w: Win, _ at: CGPoint) -> AXUIElement? {
    let deadline = Date().addingTimeInterval(1.5)
    var shallow: AXUIElement?
    repeat {
        var hit: AXUIElement?
        if AXUIElementCopyElementAtPosition(d.ax, Float(at.x), Float(at.y), &hit) == .success, let h = hit, pidOf(h) == d.pid {
            if let hw = element(attr(h, kAXWindowAttribute)), let f = frame(hw), !near(f, w.bounds) {
                fail("another window of the app is over that point")
            }
            if !wholeRoles.contains(text(h, kAXRoleAttribute)) { return h }
            shallow = h
        }
        usleep(100_000)
    } while Date() < deadline
    if let root = d.axWindow(w), let e = walk(root, at), !wholeRoles.contains(text(e, kAXRoleAttribute)) { return e }
    return shallow
}

func click(_ d: Driven, _ x: Double, _ y: Double) {
    let w = d.needWindow()
    guard x >= 0, y >= 0, x < Double(w.bounds.width), y < Double(w.bounds.height) else {
        fail("(\(x), \(y)) is outside the window (\(Int(w.bounds.width)) × \(Int(w.bounds.height)) points)")
    }
    if x < 80 && y < 32 { fail("that's the window's own close, minimise and zoom buttons; the driver doesn't press them") }
    d.enableWebTree()
    guard let hit = elementAt(d, w, CGPoint(x: w.bounds.minX + x, y: w.bounds.minY + y)) else { fail("nothing there") }
    let chain = ancestors(hit)
    if chain.contains(where: { windowControls.contains(text($0, kAXSubroleAttribute)) }) {
        fail("that's the window's own close, minimise and zoom buttons; the driver doesn't press them")
    }
    let before = picture(w)
    if let field = chain.prefix(3).first(where: isTextEntry) {
        let start = Date()
        AXUIElementSetAttributeValue(field, kAXFocusedAttribute as CFString, kCFBooleanTrue)
        print("put the cursor in the \(text(field, kAXRoleDescriptionAttribute).isEmpty ? "field" : text(field, kAXRoleDescriptionAttribute)); \(changed(d, w, from: before, since: start))")
        return
    }
    let pressable = chain.prefix(while: { !wholeRoles.contains(text($0, kAXRoleAttribute)) })
    guard let target = pressable.first(where: { actions($0).contains(kAXPressAction) }) else { fail("nothing to press there") }
    for label in [text(target, kAXTitleAttribute), text(target, kAXDescriptionAttribute)] where !label.isEmpty {
        if let refusal = pressRefusal(label) { fail(refusal) }
    }
    let start = Date()
    let result = AXUIElementPerformAction(target, kAXPressAction as CFString)
    guard result == .success else { fail("the press was refused by the app (\(result.rawValue))") }
    print(changed(d, w, from: before, since: start))
}

// MARK: keys, posted to the pid only

/// Text it types exactly: printable characters only. A line break or a tab
/// would act (send, move focus), so they're `key` presses instead.
func typeRefusal(_ s: String) -> String? {
    if s.isEmpty { return "nothing to type" }
    if s.count > 2000 { return "more than 2000 characters" }
    for ch in s {
        for u in ch.unicodeScalars {
            switch u.properties.generalCategory {
            case .control, .format, .surrogate, .privateUse, .unassigned, .lineSeparator, .paragraphSeparator:
                let code = "U+" + String(u.value, radix: 16, uppercase: true)
                return "it can't type \(code) exactly (a line break, tab or other control: `key return`, `key tab`)"
            default: break
            }
        }
        if ch.utf16.count > 20 { return "a character too long for one key event" }
    }
    return nil
}

struct KeyCombo { let code: CGKeyCode; let flags: CGEventFlags }

let namedKeys: [String: CGKeyCode] = [
    "return": 36, "enter": 76, "escape": 53, "esc": 53, "tab": 48, "space": 49,
    "delete": 51, "backspace": 51, "forwarddelete": 117, "left": 123, "right": 124,
    "down": 125, "up": 126, "home": 115, "end": 119, "pageup": 116, "pagedown": 121,
]
let modifierNames: [String: CGEventFlags] = [
    "cmd": .maskCommand, "command": .maskCommand, "shift": .maskShift, "alt": .maskAlternate,
    "option": .maskAlternate, "opt": .maskAlternate, "ctrl": .maskControl, "control": .maskControl,
]

/// The key that types `ch` on the current keyboard layout (so cmd+n is the
/// N key on any layout).
func layoutKey(_ ch: Character) -> CGKeyCode? {
    for source in [TISCopyCurrentKeyboardLayoutInputSource(), TISCopyCurrentASCIICapableKeyboardLayoutInputSource()] {
        guard let src = source?.takeRetainedValue(),
              let ptr = TISGetInputSourceProperty(src, kTISPropertyUnicodeKeyLayoutData) else { continue }
        let data = Unmanaged<CFData>.fromOpaque(ptr).takeUnretainedValue() as Data
        let found: CGKeyCode? = data.withUnsafeBytes { raw in
            guard let layout = raw.bindMemory(to: UCKeyboardLayout.self).baseAddress else { return nil }
            for code in 0..<128 {
                var dead: UInt32 = 0, length = 0
                var chars = [UniChar](repeating: 0, count: 4)
                let status = UCKeyTranslate(layout, UInt16(code), UInt16(kUCKeyActionDown), 0, UInt32(LMGetKbdType()),
                                            OptionBits(kUCKeyTranslateNoDeadKeysBit), &dead, 4, &length, &chars)
                if status == noErr, length == 1, String(utf16CodeUnits: chars, count: 1) == String(ch) { return CGKeyCode(code) }
            }
            return nil
        }
        if let found { return found }
    }
    return nil
}

/// "return", "cmd+n", "shift+tab": named keys, or one letter, digit or
/// mark, with modifiers. Quit, close, hide and minimise are refused.
func parseKey(_ spec: String, layout: (Character) -> CGKeyCode? = layoutKey) -> (KeyCombo?, String?) {
    let parts = spec.lowercased().split(separator: "+", omittingEmptySubsequences: false).map(String.init)
    guard let base = parts.last, !base.isEmpty, !parts.dropLast().contains("") else {
        return (nil, "\"\(spec)\" isn't a key (return, escape, tab, cmd+n …)")
    }
    var flags = CGEventFlags()
    for m in parts.dropLast() {
        guard let f = modifierNames[m] else { return (nil, "\"\(m)\" isn't a modifier (cmd, shift, option, ctrl)") }
        flags.insert(f)
    }
    if flags.contains(.maskCommand), ["q", "w", "h", "m", "tab", "space", "escape", "esc"].contains(base) {
        return (nil, "\(spec) quits, closes, hides or minimises, or is the system's: the driver doesn't send it")
    }
    if let code = namedKeys[base] { return (KeyCombo(code: code, flags: flags), nil) }
    guard base.count == 1, let ch = base.first, ch.isASCII, ch.isLetter || ch.isNumber || ch.isPunctuation || ch.isSymbol else {
        return (nil, "\"\(base)\" isn't a key it knows (return, escape, tab, space, delete, arrows, home, end, pageup, pagedown, or one letter or digit)")
    }
    guard let code = layout(ch) else { return (nil, "no key types \"\(ch)\" on this keyboard layout") }
    return (KeyCombo(code: code, flags: flags), nil)
}

let keySource = CGEventSource(stateID: .privateState)

/// One key, down and up, posted to the pid alone, after rule 1 is read
/// again (the same Claude app process is still there).
func post(_ d: Driven, _ code: CGKeyCode, _ flags: CGEventFlags, _ chars: [UniChar]? = nil) {
    guard identityRefusal(d.pid) == nil, executablePath(d.pid) == d.exe else {
        fail("pid \(d.pid) is no longer that Claude app; nothing more is sent")
    }
    for down in [true, false] {
        guard let e = CGEvent(keyboardEventSource: keySource, virtualKey: code, keyDown: down) else { fail("no key event") }
        e.flags = flags
        if let chars { e.keyboardSetUnicodeString(stringLength: chars.count, unicodeString: chars) }
        e.postToPid(d.pid)
    }
    usleep(8_000)
}

func needPostAccess() {
    // Preflight only: asking would put up a prompt, and that's the maintainer's.
    guard CGPreflightPostEventAccess() else { fail("this terminal may not post key events (granting it is the maintainer's step)") }
}

func typeText(_ d: Driven, _ s: String) {
    if let refusal = typeRefusal(s) { fail(refusal) }
    needPostAccess()
    let w = d.needWindow()
    d.enableWebTree()
    guard let focused = element(attr(d.ax, kAXFocusedUIElementAttribute)), pidOf(focused) == d.pid,
          isTextEntry(focused) || attr(focused, "AXEditableAncestor") != nil else {
        fail("no text field has the cursor in that window (click one first; an app in the background may not say: `front`)")
    }
    if let fw = element(attr(focused, kAXWindowAttribute)), let f = frame(fw), !near(f, w.bounds) {
        fail("the cursor is in another window of the app")
    }
    let before = picture(w)
    let start = Date()
    for ch in s { post(d, 0, [], Array(String(ch).utf16)) }
    print(changed(d, w, from: before, since: start))
}

func key(_ d: Driven, _ spec: String) {
    let (combo, refusal) = parseKey(spec)
    guard let combo else { fail(refusal ?? "not a key") }
    needPostAccess()
    let w = d.needWindow()
    let before = picture(w)
    let start = Date()
    post(d, combo.code, combo.flags)
    print(changed(d, w, from: before, since: start))
}

// MARK: menus

/// What a menu bar, menu bar item or menu item opens: its items.
func menuItems(_ e: AXUIElement) -> [AXUIElement] {
    let kids = children(e)
    if let menu = kids.first(where: { text($0, kAXRoleAttribute) == "AXMenu" }) { return children(menu) }
    return text(e, kAXRoleAttribute) == "AXMenuBar" ? kids : []
}

/// Items that quit, close, hide, sign out or reach the system (the Apple
/// menu, Services) are refused.
func menuRefusal(_ path: [String]) -> String? {
    if let first = path.first, first == "Apple" { return "the Apple menu is the system's: the driver doesn't press it" }
    if path.contains("Services") { return "Services act on other apps: the driver doesn't press them" }
    guard let leaf = path.last?.lowercased() else { return "no item named" }
    for p in ["quit", "close", "hide", "minimi", "log out", "logout", "log off", "sign out", "uninstall",
              "check for update", "restart", "shut down", "force quit", "delete", "reset"] where leaf.hasPrefix(p) {
        return "\"\(path.last!)\" quits, closes, hides, signs out or deletes: the driver doesn't press it"
    }
    return nil
}

/// Titles as written, with "..." read as the menu's "…".
func menuTitle(_ s: Substring) -> String {
    var t = s.trimmingCharacters(in: .whitespaces)
    if t.hasSuffix("...") { t = String(t.dropLast(3)) + "…" }
    return t
}

func describe(_ item: AXUIElement) -> String {
    let title = text(item, kAXTitleAttribute)
    var line = "  \(title)"
    if !menuItems(item).isEmpty { line += " ›" }
    if !(text(item, "AXMenuItemMarkChar")).isEmpty { line += "  ✓" }
    if (attr(item, kAXEnabledAttribute) as? Bool) == false { line += "  (greyed out)" }
    return line
}

func menu(_ d: Driven, _ spec: String) {
    guard let bar = element(attr(d.ax, kAXMenuBarAttribute)) else { fail("the app shows no menu bar to accessibility") }
    let path = spec.isEmpty ? [] : spec.split(separator: ">").map(menuTitle)
    var current = bar
    for (i, step) in path.enumerated() {
        var items = menuItems(current)
        if i == 0, let apple = items.first, text(apple, kAXTitleAttribute) == "Apple" { items.removeFirst() }
        let matches = items.filter { text($0, kAXTitleAttribute) == step }
        guard matches.count == 1 else {
            let at = i == 0 ? "the menu bar" : path[..<i].joined(separator: ">")
            fail(matches.isEmpty ? "no item titled exactly \"\(step)\" in \(at) (`menu \(i == 0 ? "" : "\"\(at)\"")` lists them)"
                                 : "more than one item titled \"\(step)\" in \(at)")
        }
        current = matches[0]
    }
    let items = menuItems(current)
    if !items.isEmpty {
        // a menu, not an item: list it
        for (i, item) in items.enumerated() where !text(item, kAXTitleAttribute).isEmpty {
            if path.isEmpty && i == 0 && text(item, kAXTitleAttribute) == "Apple" { continue }
            print(describe(item))
        }
        return
    }
    guard !path.isEmpty else { fail("the menu bar is empty") }
    if let refusal = menuRefusal(path) { fail(refusal) }
    if (attr(current, kAXEnabledAttribute) as? Bool) == false { fail("\"\(path.last!)\" is greyed out") }
    let w = d.needWindow()
    let before = picture(w)
    let start = Date()
    let result = AXUIElementPerformAction(current, kAXPressAction as CFString)
    guard result == .success else { fail("the press was refused by the app (\(result.rawValue))") }
    print(changed(d, w, from: before, since: start))
}

// MARK: bringing it forward, shots, the list

func front(_ d: Driven) {
    let start = Date()
    AXUIElementSetAttributeValue(d.ax, kAXFrontmostAttribute as CFString, kCFBooleanTrue)
    if let main = element(attr(d.ax, kAXMainWindowAttribute)) ?? (attr(d.ax, kAXWindowsAttribute) as? [AXUIElement])?.first {
        AXUIElementPerformAction(main, kAXRaiseAction as CFString)
    }
    while Date().timeIntervalSince(start) < 3 {
        // asked of the app (NSWorkspace's answer goes stale without a run loop)
        if d.mainWindow() != nil, (attr(d.ax, kAXFrontmostAttribute) as? Bool) == true {
            print("in front in \(Int(Date().timeIntervalSince(start) * 1000)) ms")
            return
        }
        usleep(50_000)
    }
    fail("it didn't come forward in 3 s")
}

func shot(_ d: Driven) {
    let w = d.needWindow()
    try? FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)
    let file = out.appendingPathComponent("claude-\(Int(Date().timeIntervalSince1970 * 1000)).png")
    guard capture(w, to: file) else { fail("the capture failed (is the window on another Space?)") }
    // at its size in points (a Retina capture is resampled), so a picture's
    // pixel is a click's point
    let task = Process()
    task.executableURL = URL(fileURLWithPath: "/usr/bin/sips")
    task.arguments = ["-z", "\(Int(w.bounds.height))", "\(Int(w.bounds.width))", file.path]
    task.standardOutput = FileHandle.nullDevice
    try? task.run()
    task.waitUntilExit()
    print(file.path)
    print("\(Int(w.bounds.width)) × \(Int(w.bounds.height)) points (a picture's pixel is one point: click at what you read)")
}

/// The Claude app processes, naming only AIJ_APP_PROFILE's.
func list() {
    let profile = env["AIJ_APP_PROFILE"]
    let apps = NSWorkspace.shared.runningApplications.filter { $0.bundleIdentifier == claudeBundleID }
    if apps.isEmpty { print("no Claude app is running"); return }
    for app in apps.sorted(by: { $0.processIdentifier < $1.processIdentifier }) {
        let pid = app.processIdentifier
        if let refusal = identityRefusal(pid) { print("pid \(pid): refused: \(refusal)"); continue }
        let argv = arguments(pid)
        let which: String
        if let profile, profileRefusal(argv: argv, profile: profile) == nil {
            which = profile
        } else {
            which = userDataDirs(argv ?? []).values.isEmpty ? "the default profile" : "another profile"
        }
        let wins = windowsOnScreen(pid)
        let window = wins.max { $0.bounds.width * $0.bounds.height < $1.bounds.width * $1.bounds.height }
            .map { "window \(Int($0.bounds.width)) × \(Int($0.bounds.height)) on screen" } ?? "no window on screen"
        print("pid \(pid): \(which), \(window)")
    }
}

// MARK: the selftest: the refusals, without a Claude app

func selftest() -> Never {
    var failures = 0
    func check(_ name: String, _ ok: Bool) {
        print("\(ok ? "ok  " : "FAIL") \(name)")
        if !ok { failures += 1 }
    }

    // rule 1, on paths
    check("Claude.app's own executable passes", claudeExecutableRefusal("/Applications/Claude.app/Contents/MacOS/Claude") == nil)
    check("AI Juggler is refused", claudeExecutableRefusal("/Applications/AI Juggler.app/Contents/MacOS/juggler-app") != nil)
    check("AI Juggler Dev is refused", claudeExecutableRefusal("/x/AI Juggler Dev.app/Contents/MacOS/juggler-app") != nil)
    check("Terminal is refused", claudeExecutableRefusal("/System/Applications/Utilities/Terminal.app/Contents/MacOS/Terminal") != nil)
    check("the Claude Code CLI (lowercase claude.app) is refused",
          claudeExecutableRefusal("/Users/u/Library/Application Support/Claude/claude-code/2.1/abc/claude.app/Contents/MacOS/claude") != nil)
    check("a helper inside Claude.app is refused",
          claudeExecutableRefusal("/Applications/Claude.app/Contents/Frameworks/Claude Helper (Renderer).app/Contents/MacOS/Claude Helper (Renderer)") != nil)
    check("a look-alike folder is refused", claudeExecutableRefusal("/tmp/Claude.app.x/Contents/MacOS/Claude") != nil)
    check("a plain executable is refused", claudeExecutableRefusal("/bin/zsh") != nil)

    // rule 1, on live processes (read-only: their executable's path)
    check("this process (not Claude) is refused", identityRefusal(getpid()) != nil)
    check("pid 1, 0 and -1 are refused", identityRefusal(1) != nil && identityRefusal(0) != nil && identityRefusal(-1) != nil)
    check("a pid past the system's range is refused", identityRefusal(999_999) != nil)
    var gone: pid_t = 0
    for _ in 0..<5 {
        let t = Process()
        t.executableURL = URL(fileURLWithPath: "/usr/bin/true")
        try? t.run()
        t.waitUntilExit()
        if kill(t.processIdentifier, 0) != 0 && errno == ESRCH { gone = t.processIdentifier; break }
    }
    check("a missing pid (a process that ended) is refused", gone > 1 && identityRefusal(gone) != nil)
    if let finder = NSRunningApplication.runningApplications(withBundleIdentifier: "com.apple.finder").first {
        check("a running app that isn't Claude (Finder) is refused", identityRefusal(finder.processIdentifier) != nil)
    }

    // rule 2
    let exe = "/Applications/Claude.app/Contents/MacOS/Claude"
    let root = "/Users/u/Library/Application Support/ClaudeProfiles"
    check("the matching profile passes", profileRefusal(argv: [exe, "--user-data-dir=\(root)/aij-test"], profile: "aij-test") == nil)
    check("a trailing slash passes", profileRefusal(argv: [exe, "--user-data-dir=\(root)/aij-test/"], profile: "aij-test") == nil)
    check("a two-folder profile passes", profileRefusal(argv: [exe, "--user-data-dir=\(root)/aij-test"], profile: "ClaudeProfiles/aij-test") == nil)
    check("a mismatch is refused", profileRefusal(argv: [exe, "--user-data-dir=\(root)/someone"], profile: "aij-test") != nil)
    check("a name that only ends the same is refused", profileRefusal(argv: [exe, "--user-data-dir=\(root)/not-aij-test"], profile: "aij-test") != nil)
    check("a folder inside the profile is refused", profileRefusal(argv: [exe, "--user-data-dir=\(root)/aij-test/x"], profile: "aij-test") != nil)
    check("a different parent is refused", profileRefusal(argv: [exe, "--user-data-dir=/elsewhere/aij-test"], profile: "ClaudeProfiles/aij-test") != nil)
    check("the default profile (no --user-data-dir) is refused", profileRefusal(argv: [exe], profile: "aij-test") != nil)
    check("a value in the next argument (not read by the app) is refused",
          profileRefusal(argv: [exe, "--user-data-dir", "\(root)/aij-test"], profile: "aij-test") != nil)
    check("two --user-data-dir are refused",
          profileRefusal(argv: [exe, "--user-data-dir=\(root)/aij-test", "--user-data-dir=\(root)/other"], profile: "aij-test") != nil)
    check("one after \"--\" (not a switch) is refused", profileRefusal(argv: [exe, "--", "--user-data-dir=\(root)/aij-test"], profile: "aij-test") != nil)
    check("no AIJ_APP_PROFILE is refused", profileRefusal(argv: [exe, "--user-data-dir=\(root)/aij-test"], profile: nil) != nil)
    check("an empty or dotted AIJ_APP_PROFILE is refused",
          profileRefusal(argv: [exe, "--user-data-dir=\(root)/aij-test"], profile: "") != nil
          && profileRefusal(argv: [exe, "--user-data-dir=/"], profile: "/") != nil
          && profileRefusal(argv: [exe, "--user-data-dir=\(root)/.."], profile: "..") != nil)
    check("unreadable arguments are refused", profileRefusal(argv: nil, profile: "aij-test") != nil)
    check("this process's argv reads back", arguments(getpid())?.last == "--selftest")

    // what it types and presses
    check("harmless prompts type", typeRefusal("ls") == nil && typeRefusal("echo lab") == nil
          && typeRefusal("write the word lab to lab.txt") == nil && typeRefusal("čćžšđ é") == nil)
    check("a line break, tab, bell or zero-width space is refused",
          typeRefusal("a\nb") != nil && typeRefusal("a\tb") != nil && typeRefusal("\u{7}") != nil && typeRefusal("a\u{200B}b") != nil)
    check("nothing to type is refused", typeRefusal("") != nil)
    let fakeLayout: (Character) -> CGKeyCode? = { $0 == "n" ? 45 : nil }
    check("named keys parse", ["return", "escape", "tab", "shift+tab", "space", "delete", "up", "pagedown"].allSatisfy { parseKey($0, layout: fakeLayout).0 != nil })
    check("cmd+n parses", parseKey("cmd+n", layout: fakeLayout).0.map { $0.code == 45 && $0.flags.contains(.maskCommand) } ?? false)
    check("cmd+q, cmd+w, cmd+h, cmd+m and cmd+shift+w are refused",
          ["cmd+q", "cmd+w", "cmd+h", "cmd+m", "cmd+shift+w", "cmd+tab"].allSatisfy { parseKey($0, layout: fakeLayout).0 == nil })
    check("unknown keys and modifiers are refused",
          ["hyper+x", "", "cmd+", "+n", "f13", "cmd+é"].allSatisfy { parseKey($0, layout: fakeLayout).0 == nil })
    check("a new-session menu item passes", menuRefusal(["File", "New Code Session"]) == nil)
    check("quit, close, hide, sign out, Services and the Apple menu are refused",
          menuRefusal(["Claude", "Quit Claude"]) != nil && menuRefusal(["File", "Close Window"]) != nil
          && menuRefusal(["Claude", "Hide Claude"]) != nil && menuRefusal(["Claude", "Log Out"]) != nil
          && menuRefusal(["Claude", "Services", "Anything"]) != nil && menuRefusal(["Apple", "Restart…"]) != nil)
    check("presses that sign out or quit are refused",
          pressRefusal("Log out") != nil && pressRefusal("Sign out of all devices") != nil && pressRefusal("Send") == nil)

    // the command end to end: each refused before anything is sent
    let me = executablePath(getpid()) ?? CommandLine.arguments[0]
    let scratch = (NSTemporaryDirectory() as NSString).appendingPathComponent("aij-app-drive-selftest-\(getpid())")
    func run(_ extra: [String: String], _ args: [String]) -> (Int32, String) {
        let t = Process()
        t.executableURL = URL(fileURLWithPath: me)
        t.arguments = args
        var e = env
        e["AIJ_APP_PID"] = nil
        e["AIJ_APP_PROFILE"] = nil
        e["AIJ_APP_OUT"] = scratch
        for (k, v) in extra { e[k] = v }
        t.environment = e
        let pipe = Pipe()
        t.standardError = pipe
        t.standardOutput = FileHandle.nullDevice
        do { try t.run() } catch { return (-1, "didn't start") }
        let said = String(decoding: pipe.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)
        t.waitUntilExit()
        return (t.terminationStatus, said)
    }
    let mine = "\(getpid())"
    let cases: [(String, [String: String], [String], String)] = [
        ("no AIJ_APP_PID: refused", [:], ["click", "10", "10"], "set AIJ_APP_PID"),
        ("a pid that isn't a number: refused", ["AIJ_APP_PID": "12x"], ["key", "return"], "isn't a pid"),
        ("a non-Claude pid: click refused", ["AIJ_APP_PID": mine, "AIJ_APP_PROFILE": "aij-test"], ["click", "10", "10"], "not the Claude app"),
        ("a non-Claude pid: type refused", ["AIJ_APP_PID": mine, "AIJ_APP_PROFILE": "aij-test"], ["type", "lab"], "not the Claude app"),
        ("a non-Claude pid: key refused", ["AIJ_APP_PID": mine, "AIJ_APP_PROFILE": "aij-test"], ["key", "return"], "not the Claude app"),
        ("a non-Claude pid: menu refused", ["AIJ_APP_PID": mine, "AIJ_APP_PROFILE": "aij-test"], ["menu", "File>New"], "not the Claude app"),
        ("a non-Claude pid: shot refused", ["AIJ_APP_PID": mine, "AIJ_APP_PROFILE": "aij-test"], ["shot"], "not the Claude app"),
        ("a non-Claude pid: front refused", ["AIJ_APP_PID": mine, "AIJ_APP_PROFILE": "aij-test"], ["front"], "not the Claude app"),
        ("a missing pid: refused", ["AIJ_APP_PID": "\(gone > 1 ? gone : 999_999)", "AIJ_APP_PROFILE": "aij-test"], ["click", "1", "1"], "no process"),
        ("an unknown verb: refused", [:], ["drag"], "unknown verb"),
    ]
    for (name, extra, args, words) in cases {
        let (status, said) = run(extra, args)
        check("\(name) (exit \(status))", status == 2 && said.contains(words))
    }
    try? FileManager.default.removeItem(atPath: scratch)

    // a Claude app that's running, read-only (its path and its argv's
    // --user-data-dir; nothing is sent): rule 1 passes, a wrong profile is refused
    let running = NSWorkspace.shared.runningApplications.filter { $0.bundleIdentifier == claudeBundleID }
    if let app = running.first {
        let pid = app.processIdentifier
        check("a running Claude app passes rule 1", identityRefusal(pid) == nil)
        check("a running Claude app on another profile is refused by rule 2",
              profileRefusal(argv: arguments(pid), profile: "aij-selftest-no-such-profile") != nil)
    } else {
        print("(no Claude app is running: its live checks skipped)")
    }

    print(failures == 0 ? "app_drive selftest: ok" : "app_drive selftest: \(failures) failed")
    exit(failures == 0 ? 0 : 1)
}

// MARK: the verbs

let args = Array(CommandLine.arguments.dropFirst())
guard let verb = args.first else { fail("say a verb: list, shot, click, type, key, menu, front, --selftest") }
func number(_ i: Int) -> Double {
    guard i < args.count, let v = Double(args[i]), v.isFinite else { fail("\(verb) needs numbers: \(verb) X Y") }
    return v
}
switch verb {
case "--selftest", "selftest":
    selftest()
case "list":
    list()
case "shot":
    shot(guardProcess(needsAccessibility: false))
case "click":
    guard args.count == 3 else { fail("click X Y") }
    click(guardProcess(needsAccessibility: true), number(1), number(2))
case "type":
    guard args.count >= 2 else { fail("type TEXT") }
    typeText(guardProcess(needsAccessibility: true), args.dropFirst().joined(separator: " "))
case "key":
    guard args.count == 2 else { fail("key NAME (return, escape, tab, cmd+n …)") }
    key(guardProcess(needsAccessibility: true), args[1])
case "menu":
    menu(guardProcess(needsAccessibility: true), args.dropFirst().joined(separator: " "))
case "front":
    front(guardProcess(needsAccessibility: true))
default:
    fail("unknown verb \(verb): list, shot, click, type, key, menu, front, --selftest")
}
