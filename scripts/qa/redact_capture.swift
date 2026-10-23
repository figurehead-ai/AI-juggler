// Redacts a walk capture and writes its sidecar. Fails closed.
//
//   swift scripts/qa/redact_capture.swift IN OUT.jpg --box X,Y,W,H [--box …] [--note TEXT]
//   swift scripts/qa/redact_capture.swift IN OUT.jpg --no-real-data REASON
//
// Boxes are pixels from the top-left of IN. Every box must parse and lie
// inside the image, and there must be at least one, unless --no-real-data
// says why none is needed (then the image is copied as it is). Anything
// else, and nothing is written; the exit status is non-zero.
//
// OUT.jpg.redacted.json records the output's SHA-256, the boxes and
// `checked_by_eye: false`. Open the image, then confirm it with
// `scripts/qa/check_captures.py --confirm OUT.jpg`; the evidence gate
// refuses a walk image without a confirmed sidecar whose hash matches.
import CoreGraphics
import CryptoKit
import Foundation
import ImageIO
import UniformTypeIdentifiers

func fail(_ message: String) -> Never {
  FileHandle.standardError.write(Data("redact_capture: \(message)\n".utf8))
  exit(2)
}

var args = Array(CommandLine.arguments.dropFirst())
guard args.count >= 2 else { fail("usage: IN OUT.jpg --box X,Y,W,H [--box …] [--note TEXT] | --no-real-data REASON") }
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

guard let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: input) as CFURL, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else { fail("cannot read \(input)") }
let width = image.width, height = image.height
for box in boxes where box[0] + box[2] > width || box[1] + box[3] > height {
  fail("box \(box) lies outside the \(width)x\(height) image")
}
guard let ctx = CGContext(data: nil, width: width, height: height, bitsPerComponent: 8, bytesPerRow: 0,
                          space: CGColorSpaceCreateDeviceRGB(),
                          bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { fail("no drawing context") }
ctx.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
ctx.setFillColor(CGColor(red: 0.35, green: 0.35, blue: 0.4, alpha: 1))
for box in boxes {
  ctx.fill(CGRect(x: box[0], y: height - box[1] - box[3], width: box[2], height: box[3]))
}
guard let result = ctx.makeImage() else { fail("no image") }

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
  "boxes": boxes,
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
print("\(output): \(boxes.count) box(es), sha256 \(digest.prefix(12))…; open it, then: scripts/qa/check_captures.py --confirm \(output)")
