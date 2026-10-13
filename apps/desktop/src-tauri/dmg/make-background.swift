// Draws the .dmg window background: the drag to Applications, and how to open Kopi the first
// time, since it isn't notarized. Writes background.tiff with 1x and 2x images, so it stays sharp
// on Retina screens. The icon positions and window size in tauri.conf.json match the layout here.
//
//   swift apps/desktop/src-tauri/dmg/make-background.swift

import AppKit

let width: CGFloat = 660
let height: CGFloat = 400
let appCenter = CGPoint(x: 170, y: 155)
let applicationsCenter = CGPoint(x: 490, y: 155)

// The website's light palette.
func color(_ hex: UInt32) -> NSColor {
    NSColor(
        srgbRed: CGFloat((hex >> 16) & 0xff) / 255,
        green: CGFloat((hex >> 8) & 0xff) / 255,
        blue: CGFloat(hex & 0xff) / 255,
        alpha: 1
    )
}
let background = color(0xfaf6ef)
let surface = color(0xf3ebdf)
let border = color(0xe6dacb)
let foreground = color(0x2b1d14)
let muted = color(0x6b5444)
let accent = color(0xd08a3a)

func draw(_ text: String, at point: CGPoint, size: CGFloat, weight: NSFont.Weight = .regular,
          color: NSColor = foreground, centered: Bool = false) -> CGFloat {
    let attributes: [NSAttributedString.Key: Any] = [
        .font: NSFont.systemFont(ofSize: size, weight: weight),
        .foregroundColor: color,
    ]
    let string = NSAttributedString(string: text, attributes: attributes)
    var origin = point
    if centered { origin.x -= string.size().width / 2 }
    string.draw(at: origin)
    return string.size().width
}

func drawBackground() {
    background.setFill()
    NSRect(x: 0, y: 0, width: width, height: height).fill()

    _ = draw("Drag Kopi to Applications", at: CGPoint(x: width / 2, y: 30), size: 18,
         weight: .semibold, centered: true)

    // The arrow between the two icons.
    let arrow = NSBezierPath()
    arrow.lineWidth = 3
    arrow.lineCapStyle = .round
    arrow.lineJoinStyle = .round
    let start = appCenter.x + 95
    let end = applicationsCenter.x - 95
    arrow.move(to: CGPoint(x: start, y: appCenter.y))
    arrow.line(to: CGPoint(x: end, y: appCenter.y))
    arrow.move(to: CGPoint(x: end - 12, y: appCenter.y - 12))
    arrow.line(to: CGPoint(x: end, y: appCenter.y))
    arrow.line(to: CGPoint(x: end - 12, y: appCenter.y + 12))
    accent.setStroke()
    arrow.stroke()

    // How to open it the first time.
    let panel = NSRect(x: 40, y: 262, width: width - 80, height: 118)
    let path = NSBezierPath(roundedRect: panel, xRadius: 12, yRadius: 12)
    surface.setFill()
    path.fill()
    border.setStroke()
    path.lineWidth = 1
    path.stroke()

    let left = panel.minX + 20
    let heading = draw("Opening Kopi the first time", at: CGPoint(x: left, y: panel.minY + 14),
                       size: 13, weight: .semibold)
    _ = draw("Kopi isn’t notarized by Apple, so macOS blocks it once.",
             at: CGPoint(x: left + heading + 8, y: panel.minY + 14), size: 13, color: muted)
    let steps = [
        "1.  Open Kopi from Applications. When macOS says it wasn’t opened, click Done.",
        "2.  Go to System Settings → Privacy & Security, and click Open Anyway.",
        "3.  Click Open Anyway again. On macOS 14, Control-click Kopi and choose Open.",
    ]
    for (index, step) in steps.enumerated() {
        _ = draw(step, at: CGPoint(x: left, y: panel.minY + 42 + CGFloat(index) * 22), size: 13)
    }
}

func render(scale: CGFloat) -> NSBitmapImageRep {
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil, pixelsWide: Int(width * scale), pixelsHigh: Int(height * scale),
        bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
        colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
    rep.size = NSSize(width: width, height: height)
    let context = NSGraphicsContext(bitmapImageRep: rep)!
    NSGraphicsContext.saveGraphicsState()
    // Flipped, so y runs down the window as in Finder's icon positions.
    NSGraphicsContext.current = NSGraphicsContext(cgContext: context.cgContext, flipped: true)
    context.cgContext.translateBy(x: 0, y: height)
    context.cgContext.scaleBy(x: 1, y: -1)
    drawBackground()
    NSGraphicsContext.restoreGraphicsState()
    return rep
}

let directory = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
let output = directory.appendingPathComponent("background.tiff")
let data = NSBitmapImageRep.tiffRepresentationOfImageReps(in:
    [render(scale: 1), render(scale: 2)], using: .lzw, factor: 0)!
try! data.write(to: output)
print("Wrote \(output.path)")
