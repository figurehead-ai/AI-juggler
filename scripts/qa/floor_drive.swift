// AJ-E3514: the Floor persona walk's driver — the testers' only command.
//
// Drives AI Juggler Dev's Floor window by its process id only (never by
// name: the installed AI Juggler has the same process name). Events are
// posted to that pid (`CGEvent.postToPid`), so nothing else on the Mac
// gets them and the real cursor doesn't move.
//
//   AIJ_FLOOR_PID=<pid> AIJ_FLOOR_OUT=<folder> scripts/qa/floor_drive <verb> …
//
//   shot                         capture the Floor window; prints the file
//   click X Y                    a left click at window points (X, Y)
//   drag X1 Y1 X2 Y2             a left drag
//   rdrag X1 Y1 X2 Y2            a right drag (turn and tilt)
//   scroll X Y DY                a scroll of DY lines at (X, Y)
//   move X Y                     the pointer moves there (hover)
//   key NAME [cmd,shift,alt,ctrl]  one key: a letter, a digit, or
//                                return, escape, tab, space, left, right,
//                                up, down, pageup, pagedown
//   type TEXT                    the text, as typed
//
// After an action it waits until two captures 120 ms apart are the same
// (or 3 s pass) and prints "settled in N ms".

import AppKit
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

/// The Floor window of `pid`: its id and its bounds in global points.
func floorWindow() -> (id: CGWindowID, bounds: CGRect) {
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
    let mine = list.filter { ($0[kCGWindowOwnerPID as String] as? pid_t) == pid && ($0[kCGWindowLayer as String] as? Int) == 0 }
    let floor = mine.first { (($0[kCGWindowName as String] as? String) ?? "").contains("Floor") } ?? mine.first
    guard let window = floor,
          let id = window[kCGWindowNumber as String] as? CGWindowID,
          let raw = window[kCGWindowBounds as String] as? NSDictionary,
          let bounds = CGRect(dictionaryRepresentation: raw)
    else { fail("no window of pid \(pid) on screen") }
    return (id, bounds)
}

func capture(_ name: String) -> URL {
    let file = out.appendingPathComponent(name)
    let task = Process()
    task.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
    task.arguments = ["-x", "-o", "-l\(floorWindow().id)", file.path]
    try? task.run()
    task.waitUntilExit()
    return file
}

func settle() -> Int {
    let start = Date()
    var last = (try? Data(contentsOf: capture(".settle-a.png"))) ?? Data()
    while Date().timeIntervalSince(start) < 3 {
        usleep(120_000)
        let now = (try? Data(contentsOf: capture(".settle-a.png"))) ?? Data()
        if now == last { break }
        last = now
    }
    return Int(Date().timeIntervalSince(start) * 1000)
}

func point(_ x: Double, _ y: Double) -> CGPoint {
    let b = floorWindow().bounds
    return CGPoint(x: b.origin.x + x, y: b.origin.y + y)
}

func post(_ event: CGEvent?) {
    guard let event else { fail("couldn't make the event") }
    event.postToPid(pid)
    usleep(16_000)
}

func mouse(_ type: CGEventType, _ at: CGPoint, _ button: CGMouseButton = .left) {
    post(CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: at, mouseButton: button))
}

func drag(_ from: CGPoint, _ to: CGPoint, right: Bool) {
    let (down, dragged, up): (CGEventType, CGEventType, CGEventType) = right
        ? (.rightMouseDown, .rightMouseDragged, .rightMouseUp)
        : (.leftMouseDown, .leftMouseDragged, .leftMouseUp)
    let button: CGMouseButton = right ? .right : .left
    mouse(.mouseMoved, from)
    mouse(down, from, button)
    for step in 1...12 {
        let t = Double(step) / 12
        mouse(dragged, CGPoint(x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t), button)
    }
    mouse(up, to, button)
}

let codes: [String: CGKeyCode] = [
    "return": 36, "escape": 53, "tab": 48, "space": 49, "left": 123, "right": 124, "down": 125, "up": 126,
    "pageup": 116, "pagedown": 121,
    "a": 0, "s": 1, "d": 2, "f": 3, "h": 4, "g": 5, "z": 6, "x": 7, "c": 8, "v": 9, "b": 11, "q": 12, "w": 13, "e": 14,
    "r": 15, "y": 16, "t": 17, "1": 18, "2": 19, "3": 20, "4": 21, "6": 22, "5": 23, "9": 25, "7": 26, "8": 28, "0": 29,
    "o": 31, "u": 32, "i": 34, "p": 35, "l": 37, "j": 38, "k": 40, "n": 45, "m": 46,
]

func key(_ name: String, _ mods: String) {
    guard let code = codes[name.lowercased()] else { fail("unknown key \(name)") }
    var flags: CGEventFlags = []
    for mod in mods.split(separator: ",") {
        switch mod {
        case "cmd": flags.insert(.maskCommand)
        case "shift": flags.insert(.maskShift)
        case "alt": flags.insert(.maskAlternate)
        case "ctrl": flags.insert(.maskControl)
        default: fail("unknown modifier \(mod)")
        }
    }
    for down in [true, false] {
        let event = CGEvent(keyboardEventSource: nil, virtualKey: code, keyDown: down)
        event?.flags = flags
        post(event)
    }
}

var args = Array(CommandLine.arguments.dropFirst())
guard let verb = args.first else { fail("say a verb: shot, click, drag, rdrag, scroll, move, key, type") }
args.removeFirst()
let n = { (i: Int) -> Double in
    guard i < args.count, let v = Double(args[i]) else { fail("\(verb) needs numbers") }
    return v
}

switch verb {
case "shot":
    let stamp = Int(Date().timeIntervalSince1970 * 1000)
    print(capture("floor-\(stamp).png").path)
    exit(0)
case "click":
    let at = point(n(0), n(1))
    mouse(.mouseMoved, at); mouse(.leftMouseDown, at); mouse(.leftMouseUp, at)
case "drag": drag(point(n(0), n(1)), point(n(2), n(3)), right: false)
case "rdrag": drag(point(n(0), n(1)), point(n(2), n(3)), right: true)
case "scroll":
    mouse(.mouseMoved, point(n(0), n(1)))
    post(CGEvent(scrollWheelEvent2Source: nil, units: .line, wheelCount: 1, wheel1: Int32(n(2)), wheel2: 0, wheel3: 0))
case "move": mouse(.mouseMoved, point(n(0), n(1)))
case "key": key(args.first ?? "", args.count > 1 ? args[1] : "")
case "type":
    let text = args.joined(separator: " ")
    for scalar in text.utf16 {
        var unit = scalar
        for down in [true, false] {
            let event = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: down)
            event?.keyboardSetUnicodeString(stringLength: 1, unicodeString: &unit)
            post(event)
        }
    }
default: fail("unknown verb \(verb)")
}
print("settled in \(settle()) ms")
