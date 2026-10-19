// D-23: marks an AI Juggler edition's icon, so AI Juggler Nightly and
// AI Juggler Dev are told apart from AI Juggler at a glance (Dock,
// Finder, the app switcher). Draws a coloured band with the edition's
// name across the bottom of the brand icon. macOS only (CoreGraphics).
//
//   swift scripts/edition_icon.swift IN.png OUT.png LABEL RRGGBB
//
// Used by `just dev` / `just app` (Dev) and the release build (Nightly).
import AppKit
import CoreGraphics
import Foundation

let args = CommandLine.arguments
guard args.count == 5, let rgb = UInt32(args[4], radix: 16) else {
    FileHandle.standardError.write("usage: edition_icon.swift IN.png OUT.png LABEL RRGGBB\n".data(using: .utf8)!)
    exit(2)
}
let (input, output, label) = (args[1], args[2], args[3].uppercased())
guard let source = NSImage(contentsOfFile: input),
      let cg = source.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
    FileHandle.standardError.write("cannot read \(input)\n".data(using: .utf8)!)
    exit(1)
}
let (w, h) = (cg.width, cg.height)
guard let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0,
                          space: CGColorSpace(name: CGColorSpace.sRGB)!,
                          bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { exit(1) }
let full = CGRect(x: 0, y: 0, width: w, height: h)
ctx.draw(cg, in: full)

// The band: drawn only where the icon itself is opaque, so it follows the
// icon's rounded shape instead of sticking out of it.
let band = CGRect(x: 0, y: 0, width: CGFloat(w), height: CGFloat(h) * 0.24)
ctx.saveGState()
ctx.clip(to: full, mask: cg)
ctx.setFillColor(CGColor(srgbRed: CGFloat((rgb >> 16) & 0xff) / 255,
                         green: CGFloat((rgb >> 8) & 0xff) / 255,
                         blue: CGFloat(rgb & 0xff) / 255, alpha: 1))
ctx.fill(band)
ctx.restoreGState()

// The label, white and bold, centred in the band.
NSGraphicsContext.current = NSGraphicsContext(cgContext: ctx, flipped: false)
let font = NSFont.systemFont(ofSize: CGFloat(h) * 0.13, weight: .heavy)
let text = NSAttributedString(string: label, attributes: [
    .font: font,
    .foregroundColor: NSColor.white,
    .kern: CGFloat(h) * 0.006,
])
let size = text.size()
text.draw(at: CGPoint(x: (CGFloat(w) - size.width) / 2,
                      y: band.midY - size.height / 2 + font.descender / 2))
NSGraphicsContext.current = nil

guard let marked = ctx.makeImage(),
      let png = NSBitmapImageRep(cgImage: marked).representation(using: .png, properties: [:]) else { exit(1) }
try FileManager.default.createDirectory(atPath: (output as NSString).deletingLastPathComponent,
                                        withIntermediateDirectories: true)
try png.write(to: URL(fileURLWithPath: output))
