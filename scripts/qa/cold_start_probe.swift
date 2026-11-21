// AJ-E805: one cold start of a release build, timed (run by
// scripts/qa/release_cold_start.sh, which owns the throwaway home).
//
//   cold_start_probe <app bundle> <home> <timeout seconds>
//
// Opens the app as a double-click does (`open -n`, a new instance, with
// HOME set to the throwaway home and no sessions restored), finds the
// process it started (this bundle's own binary, parent launchd, not running
// before), and waits for the main window's title "AI Juggler · …": the UI
// sets it when the shell has rendered (ui/src/lib/window-title.ts), the
// same mark the first-open check uses. Titles are read through the
// Accessibility API of that pid only (an installed AI Juggler shares the
// process name); the probe checks the trust without prompting and stops
// if it's missing. Then it quits the app with a TERM (a KILL after 10 s)
// and prints one JSON line:
//
//   {"pid":…, "open_to_process_ms":…, "process_to_shell_ms":…, "open_to_shell_ms":…}
//
// Exit 0 measured; 1 the app didn't start or the title never came; 2 usage
// or no Accessibility trust.
import ApplicationServices
import Foundation

func now() -> Double { Date().timeIntervalSince1970 * 1000 }

func run(_ path: String, _ args: [String]) -> String {
    let process = Process()
    process.executableURL = URL(fileURLWithPath: path)
    process.arguments = args
    let pipe = Pipe()
    process.standardOutput = pipe
    process.standardError = FileHandle.nullDevice
    do { try process.run() } catch { return "" }
    process.waitUntilExit()
    return String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
}

/// The pids of juggler-app processes running exactly `exe`, with their parents.
func instances(of exe: String) -> [(pid: Int32, parent: Int32)] {
    run("/usr/bin/pgrep", ["-x", "juggler-app"]).split(separator: "\n").compactMap { line in
        guard let pid = Int32(line.trimmingCharacters(in: .whitespaces)) else { return nil }
        let info = run("/bin/ps", ["-o", "ppid=,comm=", "-p", String(pid)]).trimmingCharacters(in: .whitespacesAndNewlines)
        guard let space = info.firstIndex(of: " ") else { return nil }
        let parent = Int32(info[..<space].trimmingCharacters(in: .whitespaces)) ?? 0
        let comm = info[space...].trimmingCharacters(in: .whitespaces)
        return comm == exe ? (pid, parent) : nil
    }
}

func titles(of pid: Int32) -> [String] {
    let app = AXUIElementCreateApplication(pid)
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &value) == .success,
          let windows = value as? [AXUIElement] else { return [] }
    return windows.compactMap { window in
        var title: CFTypeRef?
        guard AXUIElementCopyAttributeValue(window, kAXTitleAttribute as CFString, &title) == .success else { return nil }
        return title as? String
    }
}

func quit(_ pid: Int32, exe: String) {
    for child in instances(of: exe) where child.parent == pid { kill(child.pid, SIGTERM) }
    kill(pid, SIGTERM)
    for _ in 0..<100 {
        if kill(pid, 0) != 0 { return }
        usleep(100_000)
    }
    kill(pid, SIGKILL)
}

let args = CommandLine.arguments
guard args.count == 4, let timeout = Double(args[3]) else {
    FileHandle.standardError.write("usage: cold_start_probe <app bundle> <home> <timeout seconds>\n".data(using: .utf8)!)
    exit(2)
}
let trust = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: false] as CFDictionary
guard AXIsProcessTrustedWithOptions(trust) else {
    FileHandle.standardError.write("cold-start: no Accessibility trust for this process; not asking for it\n".data(using: .utf8)!)
    exit(2)
}
// realpath(3), as `ps` names the binary: Foundation's own resolving turns
// /private/tmp into /tmp, which then never matches.
guard let real = realpath(args[1], nil) else {
    FileHandle.standardError.write("cold-start: no app at \(args[1])\n".data(using: .utf8)!)
    exit(2)
}
let app = String(cString: real)
free(real)
let exe = app + "/Contents/MacOS/juggler-app"
let before = Set(instances(of: exe).map(\.pid))

let opened = now()
_ = run("/usr/bin/open", ["-n", "--env", "AIJ_NO_RESTORE=1", "--env", "HOME=" + args[2], app])

var pid: Int32 = 0
var started = 0.0
while now() - opened < timeout * 1000 {
    if let found = instances(of: exe).first(where: { !before.contains($0.pid) && $0.parent == 1 }) {
        pid = found.pid
        started = now()
        break
    }
    usleep(5_000)
}
guard pid != 0 else {
    FileHandle.standardError.write("cold-start: the app didn't start within \(Int(timeout)) s\n".data(using: .utf8)!)
    exit(1)
}

var shell = 0.0
while now() - opened < timeout * 1000 {
    if titles(of: pid).contains(where: { $0.hasPrefix("AI Juggler ·") }) {
        shell = now()
        break
    }
    if kill(pid, 0) != 0 { break }
    usleep(5_000)
}
quit(pid, exe: exe)
guard shell > 0 else {
    FileHandle.standardError.write("cold-start: no \"AI Juggler · …\" window within \(Int(timeout)) s (pid \(pid))\n".data(using: .utf8)!)
    exit(1)
}
print("{\"pid\":\(pid),\"open_to_process_ms\":\(Int(started - opened)),\"process_to_shell_ms\":\(Int(shell - started)),\"open_to_shell_ms\":\(Int(shell - opened))}")
