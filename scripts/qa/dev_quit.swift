// 0.5.2 RW1: quits AI Juggler Dev through its own Quit: presses its app
// menu's item "Quit AI Juggler Dev" by accessibility, addressed by pid
// only (it refuses any pid that isn't an "AI Juggler Dev.app", so the
// installed AI Juggler and Nightly, the same process name juggler-app,
// are never touched). No key is sent and the pointer doesn't move; the
// item is the one ⌘Q runs (lab_drive refuses ⌘Q), so live sessions get
// Dev's own quit sheet, pressed with lab_drive afterwards.
//
//   swiftc -O -o "$S/dev_quit" scripts/qa/dev_quit.swift   # $S: a scratch folder
//   "$S/dev_quit" <Dev's pid>
import AppKit
import ApplicationServices
guard CommandLine.arguments.count == 2, let pid = Int32(CommandLine.arguments[1]),
      let app = NSRunningApplication(processIdentifier: pid),
      app.bundleURL?.lastPathComponent == "AI Juggler Dev.app" else {
  FileHandle.standardError.write("dev_quit: give the pid of an AI Juggler Dev.app\n".data(using: .utf8)!); exit(2)
}
let ax = AXUIElementCreateApplication(pid)
func attr(_ e: AXUIElement, _ name: String) -> AnyObject? {
  var v: AnyObject?; return AXUIElementCopyAttributeValue(e, name as CFString, &v) == .success ? v : nil
}
func find(_ e: AXUIElement, _ depth: Int) -> AXUIElement? {
  if (attr(e, kAXTitleAttribute) as? String) == "Quit AI Juggler Dev", (attr(e, kAXRoleAttribute) as? String) == "AXMenuItem" { return e }
  guard depth < 6, let kids = attr(e, kAXChildrenAttribute) as? [AXUIElement] else { return nil }
  for k in kids { if let f = find(k, depth + 1) { return f } }
  return nil
}
guard let bar = attr(ax, kAXMenuBarAttribute), let item = find(bar as! AXUIElement, 0) else {
  FileHandle.standardError.write("dev_quit: no Quit AI Juggler Dev item\n".data(using: .utf8)!); exit(4)
}
let r = AXUIElementPerformAction(item, kAXPressAction as CFString)
print(r == .success ? "dev_quit: pressed Quit AI Juggler Dev (pid \(pid))" : "dev_quit: press failed (\(r.rawValue))")
exit(r == .success ? 0 : 5)
