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
//   open                     opens the Floor
//   shot                     a picture of the Floor; prints its path
//   click X Y | move X Y | drag X1 Y1 X2 Y2 | rdrag X1 Y1 X2 Y2 (turn, tilt)
//   scroll X Y DY (pan) | zoom X Y DY | key NAME (a letter or digit, left,
//   right, up, down, tab, backtab, return, escape, pageup, pagedown)
//
// The main window (X and Y from its top left), through accessibility by pid:
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

func shot(_ which: Which) -> String {
    let stamp = Int(Date().timeIntervalSince1970 * 1000)
    return capture(which, "\(which == .floor ? "floor" : "main")-\(stamp).png").path
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
    if verb == "shot" { print(shot(.main)); return }
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
        guard let area = chain.first(where: { (attr($0, "AXRole") as? String) == "AXScrollArea" }),
              let bar = attr(area, "AXVerticalScrollBar"),
              let value = attr(bar as! AXUIElement, "AXValue") as? Double else { fail("nothing scrolls there") }
        let next = min(1, max(0, value + n(3) * 0.05))
        AXUIElementSetAttributeValue(bar as! AXUIElement, "AXValue" as CFString, next as CFNumber)
    default:
        fail("unknown verb main \(verb)")
    }
    print(changed(from: before, since: start))
}

// MARK: the verbs

let args = Array(CommandLine.arguments.dropFirst())
guard let verb = args.first else { fail("say a verb: open, shot, click, move, drag, rdrag, scroll, zoom, key, main …") }
switch verb {
case "shot": print(shot(.floor))
case "main": main(Array(args.dropFirst()))
case "open", "click", "move", "drag", "rdrag", "scroll", "zoom", "key":
    floor(args.joined(separator: " "))
default: fail("unknown verb \(verb)")
}
