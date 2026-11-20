// AJ-E814: a lab window's uncovered share, and a lab-owned window that
// covers it without activating anything (scripts/qa/idle_run.sh).
//
//   window_cover share PID          the PID's largest on-screen window: its
//                                   uncovered share (0-100) from the window
//                                   server's list, and its bounds
//   window_cover cover PID SECONDS  an opaque panel of our own, labelled as
//                                   a test that closes by itself, ordered
//                                   just above that window, its exact size
//                                   and place, for SECONDS (then it exits);
//                                   prints the share once ordered. It never
//                                   activates:
//                                   an accessory app, a non-activating
//                                   panel that can't become key, and nothing
//                                   else is ordered. It takes a click itself
//                                   (and does nothing): one let through
//                                   reached the window under it and made
//                                   the lab app active (AJ-E814, after-5)
//   window_cover hide PID           hides that app (NSRunningApplication)
//
// Only windows' bounds, layers, alpha and owners are read, never their
// titles or contents. The PID is a lab build's (by pid, never by name: the
// installed AI Juggler has the same process name). Run it compiled
// (idle_run.sh builds it into $TMPDIR) or with `swift window_cover.swift`.
import AppKit
import CoreGraphics

struct Win {
  let number: Int
  let pid: Int32
  let layer: Int
  let alpha: Double
  let rect: CGRect
}

func windows() -> [Win] {
  let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
  return list.compactMap { info in
    guard let number = info[kCGWindowNumber as String] as? Int,
          let pid = info[kCGWindowOwnerPID as String] as? Int32,
          let layer = info[kCGWindowLayer as String] as? Int,
          let boundsDict = info[kCGWindowBounds as String] as? NSDictionary,
          let rect = CGRect(dictionaryRepresentation: boundsDict as CFDictionary) else { return nil }
    let alpha = info[kCGWindowAlpha as String] as? Double ?? 1
    return Win(number: number, pid: pid, layer: layer, alpha: alpha, rect: rect)
  }
}

/// The pid's largest normal-layer window, and the windows in front of it
/// (the list runs front to back).
func target(_ pid: Int32) -> (Win, [Win])? {
  let all = windows()
  let mine = all.enumerated().filter { $0.element.pid == pid && $0.element.layer == 0 && $0.element.rect.width > 200 }
  guard let best = mine.max(by: { $0.element.rect.width * $0.element.rect.height < $1.element.rect.width * $1.element.rect.height }) else { return nil }
  return (best.element, Array(all[..<best.offset]))
}

/// The share of `win` no window in front of it covers (a 4-point grid).
func uncovered(_ win: Win, _ front: [Win]) -> Double {
  let step: CGFloat = 4
  var total = 0, open = 0
  let covers = front.filter { $0.alpha > 0 && $0.rect.intersects(win.rect) }.map { $0.rect }
  var y = win.rect.minY + step / 2
  while y < win.rect.maxY {
    var x = win.rect.minX + step / 2
    while x < win.rect.maxX {
      total += 1
      let point = CGPoint(x: x, y: y)
      if !covers.contains(where: { $0.contains(point) }) { open += 1 }
      x += step
    }
    y += step
  }
  return total == 0 ? 0 : Double(open) * 100 / Double(total)
}

func share(_ pid: Int32) -> String {
  guard let (win, front) = target(pid) else { return "share=none" }
  let r = win.rect
  return String(format: "share=%.1f window=%d bounds=%.0f,%.0f,%.0fx%.0f", uncovered(win, front), win.number, r.minX, r.minY, r.width, r.height)
}

final class Panel: NSPanel {
  override var canBecomeKey: Bool { false }
  override var canBecomeMain: Bool { false }
}

let args = CommandLine.arguments
guard args.count >= 3, let pid = Int32(args[2]) else {
  print("usage: window_cover share|cover|hide PID [SECONDS]")
  exit(2)
}
switch args[1] {
case "share":
  print(share(pid))
case "hide":
  guard let app = NSRunningApplication(processIdentifier: pid) else { print("no such app"); exit(1) }
  print(app.hide() ? "hidden" : "not hidden")
case "cover":
  let seconds = Double(args.count > 3 ? args[3] : "120") ?? 120
  guard let (win, _) = target(pid) else { print("no window"); exit(1) }
  let app = NSApplication.shared
  app.setActivationPolicy(.accessory)
  // Cocoa's screen space starts at the main display's bottom left.
  let mainHeight = NSScreen.screens.first?.frame.height ?? 0
  let frame = NSRect(x: win.rect.minX, y: mainHeight - win.rect.maxY, width: win.rect.width, height: win.rect.height)
  let panel = Panel(contentRect: frame, styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
  panel.isOpaque = true
  panel.backgroundColor = NSColor(calibratedWhite: 0.5, alpha: 1)
  panel.hasShadow = false
  panel.ignoresMouseEvents = false
  panel.hidesOnDeactivate = false
  panel.isReleasedWhenClosed = false
  panel.collectionBehavior = [.ignoresCycle, .fullScreenAuxiliary]
  // Says what it is to whoever sees it (the Mac is in use while it runs).
  if let content = panel.contentView {
    let label = NSTextField(labelWithString: "AI Juggler test: measuring, closes by itself in a minute")
    label.font = NSFont.systemFont(ofSize: 15, weight: .medium)
    label.textColor = .white
    label.alignment = .center
    label.translatesAutoresizingMaskIntoConstraints = false
    content.addSubview(label)
    NSLayoutConstraint.activate([
      label.centerXAnchor.constraint(equalTo: content.centerXAnchor),
      label.centerYAnchor.constraint(equalTo: content.centerYAnchor),
    ])
  }
  panel.order(.above, relativeTo: win.number)
  DispatchQueue.main.asyncAfter(deadline: .now() + 1) {
    let all = windows()
    let ours = all.firstIndex { $0.number == panel.windowNumber } ?? -1
    let theirs = all.firstIndex { $0.number == win.number } ?? -1
    print("cover=\(panel.windowNumber) at=\(ours) target_at=\(theirs) \(share(pid))")
    fflush(stdout)
  }
  DispatchQueue.main.asyncAfter(deadline: .now() + seconds) {
    panel.close()
    print("uncovered after \(Int(seconds)) s")
    exit(0)
  }
  signal(SIGTERM) { _ in exit(0) }
  signal(SIGINT) { _ in exit(0) }
  app.run()
default:
  print("unknown verb")
  exit(2)
}
