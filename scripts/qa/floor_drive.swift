// AJ-E3514: the Floor persona walk's driver — the testers' only command.
//
// Drives AI Juggler Dev by its process id only, never by name (the
// installed AI Juggler has the same process name), and never moves the
// real pointer: the Mac stays usable while the walk runs.
//
//   AIJ_FLOOR_PID=<pid> AIJ_FLOOR_SOCKET=<socket> AIJ_FLOOR_OUT=<folder> \
//     scripts/qa/floor_drive <verb> …
//
// The Floor (its own window; X and Y are points from its top left). Posted
// system events don't reach it, so these go through the Floor's test
// input (`floor-harness` builds only, `crates/juggler-app/src/features/
// floor/drive.rs`) and answer with the time to the first frame that shows
// the result:
//   open                     opens the Floor (and brings it forward)
//   shot                     a picture of the Floor at its size in points
//                            (a pixel is a click's point); prints its path
//                            and its size
//   click X Y | move X Y | drag X1 Y1 X2 Y2 | rdrag X1 Y1 X2 Y2 (turn, tilt)
//   scroll X Y DY (pan) | zoom X Y DY | key NAME (a letter or digit, left,
//   right, up, down, tab, backtab, return, escape, pageup, pagedown)
//
//   view                     lists the View menu's items (✓ chosen)
//   view ITEM                chooses one ("Labels > Attention only",
//                            "Settings › The Floor…"); the View button
//                            itself opens nothing under the walk
//
// The main window (X and Y from its top left), through accessibility by pid
// (brought forward first if a window manager put it away):
//   main shot                a picture of it
//   main click X Y           presses what's there (or puts the cursor in a
//                            text field)
//   main type TEXT           types into the field the cursor is in
//   main scroll X Y DY       scrolls what's there by DY lines (down > 0)
// These answer with the time until the window changed (up to 3 s).

import AppKit
import ApplicationServices
import CoreGraphics
import Foundation

func fail(_ message: String) -> Never {
    FileHandle.standardError.write("floor_drive: \(message)\n".data(using: .utf8)!)
    exit(2)
}

let env = ProcessInfo.processInfo.environment
guard let pidText = env["AIJ_FLOOR_PID"], let pid = pid_t(pidText) else {
    fail("set AIJ_FLOOR_PID to AI Juggler Dev's pid")
}
let out = URL(fileURLWithPath: env["AIJ_FLOOR_OUT"] ?? FileManager.default.currentDirectoryPath)
try? FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)

enum Which { case floor, main }

/// The window of `pid`: the Floor's title starts "The Floor"; the main
/// window is the other one.
func window(_ which: Which) -> (id: CGWindowID, bounds: CGRect)? {
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
    for w in list where (w[kCGWindowOwnerPID as String] as? pid_t) == pid && (w[kCGWindowLayer as String] as? Int) == 0 {
        let isFloor = ((w[kCGWindowName as String] as? String) ?? "").hasPrefix("The Floor")
        guard isFloor == (which == .floor),
              let id = w[kCGWindowNumber as String] as? CGWindowID,
              let raw = w[kCGWindowBounds as String] as? NSDictionary,
              let bounds = CGRect(dictionaryRepresentation: raw) else { continue }
        return (id, bounds)
    }
    return nil
}

func need(_ which: Which) -> (id: CGWindowID, bounds: CGRect) {
    guard let w = window(which) else {
        fail(which == .floor ? "the Floor isn't open (`open` opens it)" : "the main window isn't on screen")
    }
    return w
}

func capture(_ which: Which, _ name: String) -> URL {
    let file = out.appendingPathComponent(name)
    try? FileManager.default.removeItem(at: file)  // screencapture won't overwrite
    let task = Process()
    task.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
    task.arguments = ["-x", "-o", "-l\(need(which).id)", file.path]
    try? task.run()
    task.waitUntilExit()
    return file
}

/// A picture of the window at its size in points (a Retina screen's
/// capture is resampled), so a picture's pixel is a click's point.
func shot(_ which: Which) -> String {
    let stamp = Int(Date().timeIntervalSince1970 * 1000)
    let size = need(which).bounds.size
    let file = capture(which, "\(which == .floor ? "floor" : "main")-\(stamp).png")
    let task = Process()
    task.executableURL = URL(fileURLWithPath: "/usr/bin/sips")
    task.arguments = ["-z", "\(Int(size.height))", "\(Int(size.width))", file.path]
    task.standardOutput = FileHandle.nullDevice
    try? task.run()
    task.waitUntilExit()
    return file.path
}

/// The picture's size line: its pixels are the window's points.
func sizeLine(_ which: Which) -> String {
    let size = need(which).bounds.size
    return "\(Int(size.width)) × \(Int(size.height)) points (a picture's pixel is one point: click at what you read)"
}

// MARK: the Floor's test input

func send(_ line: String) -> String {
    guard let path = env["AIJ_FLOOR_SOCKET"] else { fail("set AIJ_FLOOR_SOCKET to the Floor's input socket") }
    let fd = socket(AF_UNIX, SOCK_STREAM, 0)
    guard fd >= 0 else { fail("no socket") }
    defer { close(fd) }
    var addr = sockaddr_un()
    addr.sun_family = sa_family_t(AF_UNIX)
    let bytes = Array(path.utf8)
    guard bytes.count < MemoryLayout.size(ofValue: addr.sun_path) else { fail("the socket's path is too long") }
    withUnsafeMutableBytes(of: &addr.sun_path) { raw in
        for (i, b) in bytes.enumerated() { raw[i] = b }
        raw[bytes.count] = 0
    }
    let connected = withUnsafePointer(to: &addr) {
        $0.withMemoryRebound(to: sockaddr.self, capacity: 1) { connect(fd, $0, socklen_t(MemoryLayout<sockaddr_un>.size)) }
    }
    guard connected == 0 else { fail("AI Juggler Dev isn't listening (was it started with the Floor's test input?)") }
    let message = Array((line + "\n").utf8)
    _ = message.withUnsafeBytes { write(fd, $0.baseAddress, $0.count) }
    var reply = [UInt8]()
    var buf = [UInt8](repeating: 0, count: 256)
    while true {
        let n = read(fd, &buf, buf.count)
        if n <= 0 { break }
        reply += buf[0..<n]
    }
    return String(decoding: reply, as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
}

/// "ok N ms" becomes "shown in N ms"; errors keep their words.
func floor(_ line: String) {
    let reply = send(line)
    if reply.hasPrefix("ok ") {
        print("shown in \(reply.dropFirst(3))")
    } else {
        fail(reply.replacingOccurrences(of: "error: ", with: ""))
    }
}

// MARK: the main window, through accessibility

let axApp = AXUIElementCreateApplication(pid)

func attr(_ e: AXUIElement, _ name: String) -> CFTypeRef? {
    var value: CFTypeRef?
    AXUIElementCopyAttributeValue(e, name as CFString, &value)
    return value
}

func element(at x: Double, _ y: Double) -> AXUIElement {
    let b = need(.main).bounds
    var found: AXUIElement?
    guard AXUIElementCopyElementAtPosition(axApp, Float(b.origin.x + x), Float(b.origin.y + y), &found) == .success,
          let e = found else { fail("nothing there") }
    return e
}

func ancestors(_ start: AXUIElement) -> [AXUIElement] {
    var all = [start]
    var e = start
    while all.count < 12, let parent = attr(e, "AXParent") {
        e = parent as! AXUIElement
        all.append(e)
    }
    return all
}

func actions(_ e: AXUIElement) -> [String] {
    var names: CFArray?
    AXUIElementCopyActionNames(e, &names)
    return (names as? [String]) ?? []
}

/// Web content has no scroll bar for accessibility: scrolls by bringing
/// into view the first element just past the window's bottom edge (or
/// the last one above its top), as a reader's "next" would.
func scrollWeb(down: Bool) -> Bool {
    let b = need(.main).bounds
    var best: (AXUIElement, Double)?
    func point(_ e: AXUIElement, _ name: String) -> CGPoint? {
        guard let v = attr(e, name) else { return nil }
        var p = CGPoint.zero
        return AXValueGetValue(v as! AXValue, .cgPoint, &p) ? p : nil
    }
    func walk(_ e: AXUIElement, _ d: Int) {
        if d > 40 { return }
        if let p = point(e, "AXPosition"), !(attr(e, "AXRole") as? String ?? "").contains("Group") {
            if down, p.y > b.maxY + 4, best.map({ p.y < $0.1 }) ?? true { best = (e, p.y) }
            if !down, p.y < b.minY + 40, p.y > -100_000, best.map({ p.y > $0.1 }) ?? true { best = (e, p.y) }
        }
        for c in (attr(e, "AXChildren") as? [AXUIElement]) ?? [] { walk(c, d + 1) }
    }
    for w in (attr(axApp, "AXWindows") as? [AXUIElement]) ?? [] where !(attr(w, "AXTitle") as? String ?? "").hasPrefix("The Floor") {
        walk(w, 0)
    }
    guard let (e, _) = best else { return false }
    return AXUIElementPerformAction(e, "AXScrollToVisible" as CFString) == .success
}

/// Waits until the main window's picture changes (up to 3 s).
func changed(from before: Data, since start: Date) -> String {
    while Date().timeIntervalSince(start) < 3 {
        usleep(30_000)
        if let now = try? Data(contentsOf: capture(.main, "changed.png")), now != before {
            try? FileManager.default.removeItem(at: out.appendingPathComponent("changed.png"))
            return "changed in \(Int(Date().timeIntervalSince(start) * 1000)) ms"
        }
    }
    try? FileManager.default.removeItem(at: out.appendingPathComponent("changed.png"))
    return "no change in 3 s"
}

func main(_ args: [String]) {
    guard let verb = args.first else { fail("main needs a verb: shot, click, type, scroll") }
    let n = { (i: Int) -> Double in
        guard i < args.count, let v = Double(args[i]) else { fail("main \(verb) needs numbers") }
        return v
    }
    if verb == "shot" {
        print(shot(.main))
        print(sizeLine(.main))
        return
    }
    let before = (try? Data(contentsOf: capture(.main, "changed.png"))) ?? Data()
    let start = Date()
    switch verb {
    case "click":
        let chain = ancestors(element(at: n(1), n(2)))
        let role = attr(chain[0], "AXRole") as? String ?? ""
        if ["AXTextField", "AXTextArea", "AXComboBox", "AXSearchField"].contains(role) {
            AXUIElementSetAttributeValue(chain[0], "AXFocused" as CFString, kCFBooleanTrue)
        } else if let target = chain.first(where: { actions($0).contains("AXPress") }) {
            AXUIElementPerformAction(target, "AXPress" as CFString)
        } else {
            fail("nothing to press there")
        }
    case "type":
        guard let focused = attr(axApp, "AXFocusedUIElement") else { fail("no field has the cursor (click one first)") }
        let field = focused as! AXUIElement
        let text = args.dropFirst().joined(separator: " ")
        let had = attr(field, "AXValue") as? String ?? ""
        guard AXUIElementSetAttributeValue(field, "AXValue" as CFString, (had + text) as CFString) == .success else {
            fail("that field doesn't take typing")
        }
    case "scroll":
        let chain = ancestors(element(at: n(1), n(2)))
        if let area = chain.first(where: { (attr($0, "AXRole") as? String) == "AXScrollArea" }),
           let bar = attr(area, "AXVerticalScrollBar"),
           let value = attr(bar as! AXUIElement, "AXValue") as? Double {
            let next = min(1, max(0, value + n(3) * 0.05))
            AXUIElementSetAttributeValue(bar as! AXUIElement, "AXValue" as CFString, next as CFNumber)
        } else if !scrollWeb(down: n(3) > 0) {
            fail("nothing scrolls there")
        }
    default:
        fail("unknown verb main \(verb)")
    }
    print(changed(from: before, since: start))
}

// MARK: keeping the Floor in front

/// A window manager (a stage manager) can stash the Floor off screen:
/// AI Juggler Dev and its Floor window are brought forward by
/// accessibility (by pid), as a user switching to it would.
/// The same for the main window (the one that isn't the Floor).
func bringMainForward() {
    // the app's own way first (a window manager follows the app)
    _ = send("front-main")
    for _ in 0..<20 where window(.main) == nil { usleep(50_000) }
    if window(.main) != nil { return }
    AXUIElementSetAttributeValue(axApp, "AXFrontmost" as CFString, kCFBooleanTrue)
    for w in (attr(axApp, "AXWindows") as? [AXUIElement]) ?? [] where !(attr(w, "AXTitle") as? String ?? "").hasPrefix("The Floor") {
        AXUIElementSetAttributeValue(w, "AXMain" as CFString, kCFBooleanTrue)
        AXUIElementPerformAction(w, "AXRaise" as CFString)
    }
    for _ in 0..<60 where window(.main) == nil { usleep(50_000) }
}

func bringFloorForward() {
    AXUIElementSetAttributeValue(axApp, "AXFrontmost" as CFString, kCFBooleanTrue)
    for w in (attr(axApp, "AXWindows") as? [AXUIElement]) ?? [] where (attr(w, "AXTitle") as? String ?? "").hasPrefix("The Floor") {
        AXUIElementSetAttributeValue(w, "AXMain" as CFString, kCFBooleanTrue)
        AXUIElementPerformAction(w, "AXRaise" as CFString)
    }
    for _ in 0..<20 where window(.floor) == nil { usleep(50_000) }
}

/// Clicks in the window's own close, minimise and zoom buttons are
/// refused: a tester never closes the office by accident.
func refuseTitleButtons(_ args: [String]) {
    guard args.first == "click", args.count >= 3, let x = Double(args[1]), let y = Double(args[2]) else { return }
    if x < 80 && y < 32 {
        fail("that's the window's own close, minimise and zoom buttons; the driver doesn't press them")
    }
}

// MARK: the verbs

let args = Array(CommandLine.arguments.dropFirst())
guard let verb = args.first else { fail("say a verb: open, shot, click, move, drag, rdrag, scroll, zoom, key, main …") }
switch verb {
case "shot":
    if window(.floor) == nil { bringFloorForward() }
    print(shot(.floor))
    print(sizeLine(.floor))
case "main":
    if window(.main) == nil { bringMainForward() }
    main(Array(args.dropFirst()))
case "view":
    let reply = send(args.joined(separator: " "))
    guard reply.hasPrefix("ok ") else { fail(reply.replacingOccurrences(of: "error: ", with: "")) }
    let lines = reply.split(separator: "\n", omittingEmptySubsequences: false)
    if lines.count > 1 { lines.dropFirst().forEach { print($0) } } else { print("chose it; shown in \(lines[0].dropFirst(3))") }
case "open":
    // a stashed new window draws nothing until it's in front
    let reply = send("open")
    bringFloorForward()
    if reply.hasPrefix("ok ") {
        print("shown in \(reply.dropFirst(3))")
    } else if window(.floor) != nil {
        print("shown (brought forward)")
    } else {
        fail(reply.replacingOccurrences(of: "error: ", with: ""))
    }
case "click", "move", "drag", "rdrag", "scroll", "zoom", "key":
    refuseTitleButtons(args)
    if window(.floor) == nil { bringFloorForward() }
    floor(args.joined(separator: " "))
default: fail("unknown verb \(verb)")
}
