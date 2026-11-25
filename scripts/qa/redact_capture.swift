// Redacts a walk capture and writes its sidecar. Fails closed.
//
//   swift scripts/qa/redact_capture.swift IN OUT.jpg --box X,Y,W,H [--box …] [--note TEXT]
//   swift scripts/qa/redact_capture.swift IN OUT.jpg --no-real-data REASON
//   swift scripts/qa/redact_capture.swift --selftest
//
// Boxes are pixels from the top-left of IN. Every box must parse and lie
// inside the image, and there must be at least one, unless --no-real-data
// says why none is needed. Anything else, and nothing is written; the exit
// status is non-zero.
//
// The Mac account's name (0.5.1 F24: a `--no-real-data` capture showed the
// account's first name in AI Juggler's profile menu). Whatever the flags
// say, the picture is read by text recognition and every line showing the
// account's full name, any word of it (the first name too) or its login
// name is covered; the result is read again, and if the name may still be
// there nothing is written. The name comes from the system's own user
// record (NSFullUserName and NSUserName: what `id -F` and `id -un` say),
// never from an argument or the environment, and is never printed or kept:
// the sidecar records only the covered boxes and how many lines they were.
// No name to look for, or text recognition failing, also writes nothing.
//
// OUT.jpg.redacted.json records the output's SHA-256, the boxes and
// `checked_by_eye: false`. Open the image, then confirm it with
// `scripts/qa/check_captures.py --confirm OUT.jpg`; the evidence gate
// refuses a walk image without a confirmed sidecar whose hash matches.
import CoreGraphics
import CoreText
import CryptoKit
import Foundation
import ImageIO
import UniformTypeIdentifiers
import Vision

func fail(_ message: String) -> Never {
  FileHandle.standardError.write(Data("redact_capture: \(message)\n".utf8))
  exit(2)
}

// MARK: - the account's name (pure parts: the selftest)

/// `text` as compared: no case, no accents, no full-width forms.
func fold(_ text: String) -> String {
  text.folding(options: [.caseInsensitive, .diacriticInsensitive, .widthInsensitive], locale: nil)
    .trimmingCharacters(in: .whitespacesAndNewlines)
}

/// What to look for, from a full name and a login name: the full name,
/// each word of it (two letters and more: a short first name counts too),
/// and the login name. Longest first. Never printed.
func nameNeedles(full: String, login: String) -> [String] {
  var out = Set<String>()
  let whole = fold(full).split(whereSeparator: { $0.isWhitespace }).joined(separator: " ")
  if whole.count >= 2 { out.insert(whole) }
  for word in whole.split(whereSeparator: { !$0.isLetter }) where word.count >= 2 { out.insert(String(word)) }
  let user = fold(login)
  if user.count >= 3 { out.insert(user) }
  return out.sorted { $0.count != $1.count ? $0.count > $1.count : $0 < $1 }
}

/// Whether `text` may show one of `needles` (pure): one of five letters or
/// more (or with a space) anywhere in it, a shorter one as a whole word
/// ("Sam" in "Sam's", not in "Sample").
func showsName(_ text: String, _ needles: [String]) -> Bool {
  let folded = fold(text)
  let words = Set(folded.split(whereSeparator: { !$0.isLetter && !$0.isNumber }).map(String.init))
  let squashed = String(folded.filter { $0.isLetter || $0.isNumber })
  return needles.contains { needle in
    if needle.count >= 5 || needle.contains(" ") {
      // Also across a recognition that split or joined the words.
      return folded.contains(needle) || squashed.contains(needle.filter { $0.isLetter || $0.isNumber })
    }
    return words.contains(needle)
  }
}

// MARK: - reading the picture

/// A line of text in the picture: what it may say (the top candidates),
/// and its box in pixels from the top-left.
struct Line { let says: [String]; let box: [Int] }

/// Every line of text recognition finds in `image`. Throws when it
/// can't read the picture (the caller then writes nothing).
func lines(_ image: CGImage) throws -> [Line] {
  // Small interface text reads better at twice the size.
  let scale = image.width < 2600 ? 2 : 1
  var read = image
  if scale > 1, let ctx = CGContext(data: nil, width: image.width * scale, height: image.height * scale, bitsPerComponent: 8,
                                     bytesPerRow: 0, space: CGColorSpaceCreateDeviceRGB(),
                                     bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) {
    ctx.interpolationQuality = .high
    ctx.draw(image, in: CGRect(x: 0, y: 0, width: image.width * scale, height: image.height * scale))
    if let big = ctx.makeImage() { read = big }
  }
  let request = VNRecognizeTextRequest()
  request.recognitionLevel = .accurate
  request.usesLanguageCorrection = false
  // Down to about 7 px text in the picture (Vision's default skips text under 1/32 of its height).
  request.minimumTextHeight = Float(min(0.03125, 7.0 / Double(max(image.height, 1))))
  try VNImageRequestHandler(cgImage: read, options: [:]).perform([request])
  guard let found = request.results else { throw NSError(domain: "redact_capture", code: 1) }
  let w = Double(image.width), h = Double(image.height), pad = 4.0
  return found.map { observation in
    let b = observation.boundingBox
    let x0 = max(0, b.minX * w - pad), y0 = max(0, (1 - b.maxY) * h - pad)
    let x1 = min(w, b.maxX * w + pad), y1 = min(h, (1 - b.minY) * h + pad)
    let box = [Int(x0.rounded(.down)), Int(y0.rounded(.down)), Int((x1 - x0).rounded(.up)), Int((y1 - y0).rounded(.up))]
    return Line(says: observation.topCandidates(3).map(\.string), box: box)
  }
}

/// The boxes of the lines in `image` that may show the name.
func nameBoxes(_ image: CGImage, _ needles: [String]) throws -> [[Int]] {
  try lines(image).filter { line in line.says.contains { showsName($0, needles) } }.map(\.box).filter { $0[2] > 0 && $0[3] > 0 }
}

/// `image` with `boxes` (pixels from the top-left) filled.
func covered(_ image: CGImage, _ boxes: [[Int]]) -> CGImage? {
  let width = image.width, height = image.height
  guard let ctx = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
                            space: CGColorSpaceCreateDeviceRGB(),
                            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
  ctx.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
  ctx.setFillColor(CGColor(red: 0.35, green: 0.35, blue: 0.4, alpha: 1))
  for box in boxes {
    ctx.fill(CGRect(x: box[0], y: height - box[1] - box[3], width: box[2], height: box[3]))
  }
  return ctx.makeImage()
}

enum Redacted { case clean(CGImage, nameBoxes: [[Int]]), nameRemains, unreadable }

/// `image` with the given boxes and every line showing the name covered,
/// read again: `.nameRemains` when the name may still show, `.unreadable`
/// when text recognition fails (either way: write nothing).
func redact(_ image: CGImage, boxes: [[Int]], needles: [String]) -> Redacted {
  guard let first = covered(image, boxes) else { return .unreadable }
  guard let found = try? nameBoxes(first, needles) else { return .unreadable }
  guard let second = covered(first, found) else { return .unreadable }
  guard let left = try? nameBoxes(second, needles) else { return .unreadable }
  return left.isEmpty ? .clean(second, nameBoxes: found) : .nameRemains
}

// MARK: - the selftest (a made-up name; nothing written)

/// A white picture with `texts` drawn in black, one per line.
func picture(_ texts: [String], width: Int = 900, lineHeight: Int = 60) -> CGImage {
  let height = lineHeight * (texts.count + 1)
  let ctx = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
                      space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
  ctx.setFillColor(CGColor(red: 1, green: 1, blue: 1, alpha: 1))
  ctx.fill(CGRect(x: 0, y: 0, width: width, height: height))
  let font = CTFontCreateWithName("Helvetica" as CFString, 26, nil)
  for (n, text) in texts.enumerated() {
    let attributed = NSAttributedString(string: text, attributes: [
      NSAttributedString.Key(kCTFontAttributeName as String): font,
      NSAttributedString.Key(kCTForegroundColorAttributeName as String): CGColor(red: 0, green: 0, blue: 0, alpha: 1),
    ])
    ctx.textPosition = CGPoint(x: 30, y: height - lineHeight * (n + 1))
    CTLineDraw(CTLineCreateWithAttributedString(attributed), ctx)
  }
  return ctx.makeImage()!
}

func selftest() -> Never {
  var failures: [String] = []
  func check(_ ok: Bool, _ what: String) { if !ok { failures.append(what) } }
  let needles = nameNeedles(full: "Samwise Rivera-Oakes", login: "someone")
  check(needles.contains("samwise rivera-oakes") && needles.contains("samwise") && needles.contains("rivera")
        && needles.contains("oakes") && needles.contains("someone"), "needles: the full name, each word and the login")
  check(nameNeedles(full: "  ", login: "").isEmpty, "needles: nothing to look for from no name")
  check(nameNeedles(full: "Al Bo", login: "").contains("al"), "needles: a two-letter first name counts")
  check(showsName("Samwise", needles) && showsName("SAMWISE · On this Mac", needles), "a line with the first name")
  check(showsName("/Users/someone/lab", needles), "a path with the login name")
  check(showsName("Sámwisé", needles), "accents don't hide it")
  check(showsName("Samwise Riv era-Oakes", needles), "a recognition that split a word")
  check(!showsName("Open now · 5 apps · 16 sessions", needles), "a line without it")
  let short = nameNeedles(full: "Sam Li", login: "")
  check(showsName("Sam's menu", short) && !showsName("Sample", short) && !showsName("Linear", short), "a short name only as a word")

  // A picture showing the made-up name: covered, the rest still readable.
  let shown = picture(["Inactive apps 5 apps", "Samwise Rivera-Oakes", "On this Mac"])
  switch redact(shown, boxes: [], needles: needles) {
  case .clean(let result, let boxes):
    check(!boxes.isEmpty, "the name's line was found")
    let after = (try? lines(result).flatMap(\.says)) ?? []
    check(!after.contains { showsName($0, needles) }, "the name is gone after")
    check(after.contains { fold($0).contains("on this mac") }, "the other lines still read")
  case .nameRemains: failures.append("the name's line was found but not covered")
  case .unreadable: failures.append("text recognition failed on the made-up picture")
  }
  // Nothing to cover: the picture as it was.
  if case .clean(_, let boxes) = redact(picture(["Home", "Open now"]), boxes: [], needles: needles) {
    check(boxes.isEmpty, "nothing covered when the name isn't there")
  } else { failures.append("a picture without the name was refused") }
  // Fail closed: a picture that still shows the name is never called clean.
  check(((try? nameBoxes(shown, needles)) ?? []).isEmpty == false, "the uncovered picture is seen to show the name")

  if failures.isEmpty { print("redact_capture: selftest passed (a made-up name; nothing written)"); exit(0) }
  for failure in failures { FileHandle.standardError.write(Data("redact_capture selftest: \(failure)\n".utf8)) }
  exit(1)
}

// MARK: - the run

var args = Array(CommandLine.arguments.dropFirst())
if args == ["--selftest"] { selftest() }
guard args.count >= 2 else { fail("usage: IN OUT.jpg --box X,Y,W,H [--box …] [--note TEXT] | --no-real-data REASON | --selftest") }
let input = args.removeFirst()
let output = args.removeFirst()
var boxes: [[Int]] = []
var note = ""
var noRealData: String? = nil
while !args.isEmpty {
  let flag = args.removeFirst()
  guard !args.isEmpty else { fail("\(flag) needs a value") }
  let value = args.removeFirst()
  switch flag {
  case "--box":
    let parts = value.split(separator: ",").map { Int($0.trimmingCharacters(in: .whitespaces)) }
    guard parts.count == 4, parts.allSatisfy({ $0 != nil }) else { fail("box '\(value)' is not X,Y,W,H") }
    let box = parts.map { $0! }
    guard box[2] > 0, box[3] > 0, box[0] >= 0, box[1] >= 0 else { fail("box '\(value)' is empty or negative") }
    boxes.append(box)
  case "--note": note = value
  case "--no-real-data":
    guard !value.trimmingCharacters(in: .whitespaces).isEmpty else { fail("--no-real-data needs a reason") }
    noRealData = value
  default: fail("unknown argument '\(flag)'")
  }
}
if boxes.isEmpty && noRealData == nil { fail("no boxes: give --box, or --no-real-data REASON") }
if !boxes.isEmpty && noRealData != nil { fail("--box and --no-real-data together") }

// The system's own user record; never an argument, never printed.
let needles = nameNeedles(full: NSFullUserName(), login: NSUserName())
if needles.isEmpty { fail("the account's name can't be read from the system's user record, so it can't be looked for; nothing written") }

guard let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: input) as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else { fail("cannot read \(input)") }
let width = image.width, height = image.height
for box in boxes where box[0] + box[2] > width || box[1] + box[3] > height {
  fail("box \(box) lies outside the \(width)x\(height) image")
}
let result: CGImage
let named: [[Int]]
switch redact(image, boxes: boxes, needles: needles) {
case .clean(let made, let found): result = made; named = found
case .nameRemains: fail("the Mac account's name may still show after covering it; nothing written")
case .unreadable: fail("text recognition couldn't read the picture, so the account's name can't be ruled out; nothing written")
}

// Write to a temporary file first, so a failure never leaves a half-made OUT.
let tmp = output + ".tmp-\(ProcessInfo.processInfo.processIdentifier)"
guard let dest = CGImageDestinationCreateWithURL(URL(fileURLWithPath: tmp) as CFURL, UTType.jpeg.identifier as CFString, 1, nil)
else { fail("cannot write \(output)") }
CGImageDestinationAddImage(dest, result, [kCGImageDestinationLossyCompressionQuality: 0.8] as CFDictionary)
guard CGImageDestinationFinalize(dest), let bytes = FileManager.default.contents(atPath: tmp) else {
  try? FileManager.default.removeItem(atPath: tmp)
  fail("cannot write \(output)")
}
let digest = SHA256.hash(data: bytes).map { String(format: "%02x", $0) }.joined()
var sidecar: [String: Any] = [
  "file": (output as NSString).lastPathComponent,
  "sha256": digest,
  "size": [width, height],
  "boxes": boxes + named,
  "account_name_lines": named.count,
  "checked_by_eye": false,
]
if !note.isEmpty { sidecar["note"] = note }
if let reason = noRealData { sidecar["no_real_data"] = reason }
guard let json = try? JSONSerialization.data(withJSONObject: sidecar, options: [.prettyPrinted, .sortedKeys]) else {
  try? FileManager.default.removeItem(atPath: tmp)
  fail("cannot encode the sidecar")
}
do {
  if FileManager.default.fileExists(atPath: output) { try FileManager.default.removeItem(atPath: output) }
  try FileManager.default.moveItem(atPath: tmp, toPath: output)
  try (json + Data("\n".utf8)).write(to: URL(fileURLWithPath: output + ".redacted.json"))
} catch {
  try? FileManager.default.removeItem(atPath: tmp)
  fail("cannot write \(output): \(error)")
}
let namedWords = named.isEmpty ? "" : "; \(named.count) line(s) showing the Mac account's name covered"
print("\(output): \(boxes.count) box(es)\(namedWords), sha256 \(digest.prefix(12))…; open it, then: scripts/qa/check_captures.py --confirm \(output)")
