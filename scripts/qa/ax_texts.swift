// ADR-0032: the words a running app's window shows, in reading order,
// read through the accessibility tree of the process with pid argv[1].
// Read-only: it never clicks, types, focuses or raises anything.
//
// Each line: <role>\t<title or description>\t<value>, depth first.
import ApplicationServices
import Foundation

guard CommandLine.arguments.count > 1, let pid = Int32(CommandLine.arguments[1]) else {
    FileHandle.standardError.write("usage: ax_texts.swift <pid>\n".data(using: .utf8)!)
    exit(2)
}
guard AXIsProcessTrusted() else {
    FileHandle.standardError.write("ax_texts: this terminal has no accessibility access\n".data(using: .utf8)!)
    exit(3)
}

func attr(_ element: AXUIElement, _ name: String) -> AnyObject? {
    var value: AnyObject?
    return AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success ? value : nil
}

func text(_ element: AXUIElement, _ name: String) -> String {
    (attr(element, name) as? String)?.replacingOccurrences(of: "\n", with: " ") ?? ""
}

var seen = 0
func walk(_ element: AXUIElement, _ depth: Int) {
    seen += 1
    if depth > 60 || seen > 40_000 { return }
    let role = text(element, kAXRoleAttribute)
    let label = [text(element, kAXTitleAttribute), text(element, kAXDescriptionAttribute)].first { !$0.isEmpty } ?? ""
    let value = text(element, kAXValueAttribute)
    if !label.isEmpty || !value.isEmpty {
        print("\(role)\t\(label)\t\(value)")
    }
    for child in (attr(element, kAXChildrenAttribute) as? [AXUIElement]) ?? [] {
        walk(child, depth + 1)
    }
}

let app = AXUIElementCreateApplication(pid)
for window in (attr(app, kAXWindowsAttribute) as? [AXUIElement]) ?? [] {
    walk(window, 0)
}
