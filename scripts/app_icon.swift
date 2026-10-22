// The AI Juggler app icon, drawn as vectors at any size: three tools in
// flight over a throw arc (the website's mark), on Apple's app-icon grid.
// The rounded square fills 824 of 1024 points with an even margin, so
// macOS shows the icon as it is rather than inside a grey frame.
//
//   swift scripts/app_icon.swift OUT.png [SIZE]
//
// SIZE defaults to 1024. The release build makes every other size from the
// 1024 PNG (`cargo tauri icon`); the editions mark it with their band
// (scripts/edition_icon.swift).
import AppKit
import CoreGraphics
import Foundation

let args = CommandLine.arguments
guard args.count == 2 || args.count == 3 else {
    FileHandle.standardError.write("usage: app_icon.swift OUT.png [SIZE]\n".data(using: .utf8)!)
    exit(2)
}
let output = args[1]
let size = args.count == 3 ? (Int(args[2]) ?? 1024) : 1024
let s = CGFloat(size) / 1024 // drawing is in 1024-point units

func rgb(_ hex: UInt32, _ alpha: CGFloat = 1) -> CGColor {
    CGColor(
        srgbRed: CGFloat((hex >> 16) & 0xFF) / 255,
        green: CGFloat((hex >> 8) & 0xFF) / 255,
        blue: CGFloat(hex & 0xFF) / 255,
        alpha: alpha
    )
}

guard let ctx = CGContext(
    data: nil, width: size, height: size, bitsPerComponent: 8, bytesPerRow: 0,
    space: CGColorSpace(name: CGColorSpace.sRGB)!,
    bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
) else { exit(1) }
ctx.scaleBy(x: s, y: s)
// CoreGraphics' origin is bottom-left: flip so y grows downward like the
// mark's own coordinates.
ctx.translateBy(x: 0, y: 1024)
ctx.scaleBy(x: 1, y: -1)

// The rounded square: Apple's grid (824 pt, radius ≈ 185 pt), with a soft
// drop shadow below it.
let tile = CGRect(x: 100, y: 100, width: 824, height: 824)
let tilePath = CGPath(roundedRect: tile, cornerWidth: 185, cornerHeight: 185, transform: nil)
ctx.saveGState()
ctx.setShadow(offset: CGSize(width: 0, height: 12), blur: 28, color: rgb(0x000000, 0.35))
ctx.addPath(tilePath)
ctx.setFillColor(rgb(0x0B0E17))
ctx.fillPath()
ctx.restoreGState()

// Its face: deep navy, a touch lighter at the top.
ctx.saveGState()
ctx.addPath(tilePath)
ctx.clip()
let face = CGGradient(
    colorsSpace: CGColorSpace(name: CGColorSpace.sRGB)!,
    colors: [rgb(0x161A2B), rgb(0x0A0C14)] as CFArray,
    locations: [0, 1]
)!
ctx.drawLinearGradient(face, start: CGPoint(x: 512, y: 100), end: CGPoint(x: 512, y: 924), options: [])

// The mark, from its 24-unit viewBox, scaled to sit inside the tile with
// room for its glow: the arc from the left ball, over the top one, to the
// right one.
let unit: CGFloat = 30
let origin = CGPoint(x: 512 - 12 * unit, y: 548 - 12 * unit)
func p(_ x: CGFloat, _ y: CGFloat) -> CGPoint { CGPoint(x: origin.x + x * unit, y: origin.y + y * unit) }

let arc = CGMutablePath()
arc.move(to: p(5.2, 17))
arc.addCurve(to: p(18.8, 17), control1: p(6.4, 2.6), control2: p(17.6, 2.6))
ctx.saveGState()
ctx.setShadow(offset: .zero, blur: 22, color: rgb(0x7C7CF4, 0.55))
ctx.addPath(arc)
ctx.setLineWidth(1.15 * unit)
ctx.setLineCap(.round)
ctx.setStrokeColor(rgb(0x7C7CF4))
ctx.strokePath()
ctx.restoreGState()

// Each ball: a halo, then the ball.
func ball(_ center: CGPoint, _ radius: CGFloat, _ hex: UInt32) {
    let halo = CGGradient(
        colorsSpace: CGColorSpace(name: CGColorSpace.sRGB)!,
        colors: [rgb(hex, 0.45), rgb(hex, 0)] as CFArray,
        locations: [0, 1]
    )!
    ctx.drawRadialGradient(
        halo, startCenter: center, startRadius: radius * 0.8,
        endCenter: center, endRadius: radius * 2.4, options: []
    )
    ctx.addEllipse(in: CGRect(x: center.x - radius, y: center.y - radius, width: radius * 2, height: radius * 2))
    ctx.setFillColor(rgb(hex))
    ctx.fillPath()
}
ball(p(5.2, 17), 2.35 * unit, 0x7C7CF4)
ball(p(12, 6.9), 2.55 * unit, 0xA5A5FB)
ball(p(18.8, 17), 2.35 * unit, 0x22D3EE)

// A hairline edge, so the tile reads on a dark Dock too.
ctx.addPath(tilePath)
ctx.setLineWidth(2)
ctx.setStrokeColor(rgb(0xFFFFFF, 0.08))
ctx.strokePath()
ctx.restoreGState()

guard let image = ctx.makeImage() else { exit(1) }
let rep = NSBitmapImageRep(cgImage: image)
guard let png = rep.representation(using: .png, properties: [:]) else { exit(1) }
do {
    try png.write(to: URL(fileURLWithPath: output))
} catch {
    FileHandle.standardError.write("cannot write \(output)\n".data(using: .utf8)!)
    exit(1)
}
