//! D-38: an app's icon as a PNG, through macOS's own safe APIs (the crate
//! forbids `unsafe`).
//!
//! - `NSWorkspace::iconForFile` gives the icon exactly as Finder and the
//!   Dock show it on this Mac: the user's own icon if they set one, and
//!   whatever style macOS draws it in. It holds one drawing per size
//!   (16 to 1024 px, at 1× and 2×).
//! - The drawing for the size asked for (exactly that wide, else the next
//!   wider) is put alone in an image and rendered to a TIFF in memory,
//!   which macOS reads back as a plain `NSBitmapImageRep`. (Drawing into
//!   a bitmap of our own needs a premultiplied one, which only a raw
//!   pointer call makes.)
//! - Its pixels are read one by one through `colorAtX_y`, in sRGB (the
//!   bitmap's memory is a raw pointer), and written as a PNG with the
//!   `png` crate (`representationUsingType` takes an untyped dictionary
//!   and is `unsafe`). That is 20 to 150 ms per app and size, off the
//!   main thread, once: the result is cached.
//!
//! Off macOS there is no icon to read, and the UI draws its letters.

/// A PNG of `width` × `height` straight-alpha RGBA `pixels`.
#[must_use]
pub fn encode_png(pixels: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let expected = usize::try_from(width).ok()? * usize::try_from(height).ok()? * 4;
    if width == 0 || height == 0 || pixels.len() != expected {
        return None;
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Balanced);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(pixels).ok()?;
        writer.finish().ok()?;
    }
    Some(out)
}

/// `pixels` (`width` × `height` RGBA) at `size` × `size`: each new pixel
/// the alpha-weighted average of the ones it covers, so edges stay clean
/// when an icon is only available larger.
#[must_use]
pub fn resample(pixels: &[u8], width: u32, height: u32, size: u32) -> Vec<u8> {
    let (w, h, s) = (width as usize, height as usize, size as usize);
    if w == 0 || h == 0 || s == 0 || pixels.len() != w * h * 4 {
        return Vec::new();
    }
    if w == s && h == s {
        return pixels.to_vec();
    }
    let mut out = vec![0u8; s * s * 4];
    for y in 0..s {
        let (y0, y1) = (
            y * h / s,
            ((y + 1) * h).div_ceil(s).max(y * h / s + 1).min(h),
        );
        for x in 0..s {
            let (x0, x1) = (
                x * w / s,
                ((x + 1) * w).div_ceil(s).max(x * w / s + 1).min(w),
            );
            let mut sum = [0u64; 4];
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let at = (sy * w + sx) * 4;
                    let alpha = u64::from(pixels[at + 3]);
                    for channel in 0..3 {
                        sum[channel] += u64::from(pixels[at + channel]) * alpha;
                    }
                    sum[3] += alpha;
                }
            }
            let count = u64::try_from((y1 - y0) * (x1 - x0)).unwrap_or(0);
            let at = (y * s + x) * 4;
            for channel in 0..3 {
                let value = sum[channel].checked_div(sum[3]).unwrap_or(0);
                out[at + channel] = u8::try_from(value).unwrap_or(u8::MAX);
            }
            let alpha = (sum[3] + count / 2).checked_div(count).unwrap_or(0);
            out[at + 3] = u8::try_from(alpha).unwrap_or(u8::MAX);
        }
    }
    out
}

/// A component in `0.0..=1.0` as a byte.
#[must_use]
pub fn byte(component: f64) -> u8 {
    // Clamped first: the cast can't overflow or wrap.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = (component.clamp(0.0, 1.0) * 255.0).round() as u8;
    value
}

#[cfg(target_os = "macos")]
mod mac {
    use std::path::{Path, PathBuf};

    use objc2::AllocAnyThread;
    use objc2::rc::{Retained, autoreleasepool};
    use objc2_app_kit::{NSBitmapImageRep, NSColorSpace, NSImage, NSImageRep, NSWorkspace};
    use objc2_foundation::{NSBundle, NSSize, NSString};

    /// Where macOS keeps the app with bundle id `id`.
    #[must_use]
    pub fn app_with_bundle_id(id: &str) -> Option<PathBuf> {
        autoreleasepool(|_| {
            let url = NSWorkspace::sharedWorkspace()
                .URLForApplicationWithBundleIdentifier(&NSString::from_str(id))?;
            Some(PathBuf::from(url.path()?.to_string()))
        })
    }

    /// The bundle id bundle `path` declares.
    #[must_use]
    pub fn bundle_id(path: &Path) -> Option<String> {
        autoreleasepool(|_| {
            let bundle = NSBundle::bundleWithPath(&NSString::from_str(path.to_str()?))?;
            Some(bundle.bundleIdentifier()?.to_string())
        })
    }

    /// The RGBA pixels of `rep` (top row first), in sRGB.
    fn pixels(rep: &NSBitmapImageRep) -> Option<(Vec<u8>, u32, u32)> {
        let (width, height) = (rep.pixelsWide(), rep.pixelsHigh());
        let (w, h) = (u32::try_from(width).ok()?, u32::try_from(height).ok()?);
        if w == 0 || h == 0 || w > 1024 || h > 1024 {
            return None;
        }
        let srgb = NSColorSpace::sRGBColorSpace();
        let mut out = Vec::with_capacity(usize::try_from(w * h * 4).ok()?);
        for y in 0..height {
            for x in 0..width {
                let Some(color) = rep.colorAtX_y(x, y) else {
                    out.extend_from_slice(&[0, 0, 0, 0]);
                    continue;
                };
                let color = color.colorUsingColorSpace(&srgb).unwrap_or(color);
                out.extend_from_slice(&[
                    super::byte(color.redComponent()),
                    super::byte(color.greenComponent()),
                    super::byte(color.blueComponent()),
                    super::byte(color.alphaComponent()),
                ]);
            }
        }
        Some((out, w, h))
    }

    /// The icon's own drawing for `px`: one exactly `px` wide, else the
    /// smallest wider one, else the widest.
    fn best_for(icon: &NSImage, px: u32) -> Option<Retained<NSImageRep>> {
        let px = isize::try_from(px).ok()?;
        let reps = icon.representations();
        let wide = |rep: &Retained<NSImageRep>| rep.pixelsWide();
        reps.iter()
            .find(|rep| wide(rep) == px)
            .or_else(|| reps.iter().filter(|rep| wide(rep) > px).min_by_key(wide))
            .or_else(|| reps.iter().max_by_key(wide))
    }

    /// `rep` alone, rendered to a bitmap of its own pixel size (through a
    /// TIFF in memory: macOS draws it, and the result is a plain bitmap).
    fn rasterised(rep: &NSImageRep) -> Option<Retained<NSBitmapImageRep>> {
        let side = f64::from(u32::try_from(rep.pixelsWide()).ok()?);
        let alone = NSImage::initWithSize(NSImage::alloc(), NSSize::new(side, side));
        alone.addRepresentation(rep);
        let tiff = alone.TIFFRepresentation()?;
        NSBitmapImageRep::imageRepWithData(&tiff)
    }

    /// Bundle `bundle`'s icon as a `px` × `px` PNG.
    #[must_use]
    pub fn icon_png(bundle: &Path, px: u32) -> Option<Vec<u8>> {
        autoreleasepool(|_| {
            let icon =
                NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(bundle.to_str()?));
            let best = best_for(&icon, px)?;
            let bitmap = rasterised(&best)?;
            let (rgba, w, h) = pixels(&bitmap)?;
            // Nothing drawn (an icon macOS couldn't render) is no icon.
            if rgba.iter().skip(3).step_by(4).all(|alpha| *alpha == 0) {
                return None;
            }
            super::encode_png(&super::resample(&rgba, w, h, px), px, px)
        })
    }
}

#[cfg(target_os = "macos")]
pub use mac::{app_with_bundle_id, bundle_id, icon_png};

/// Off macOS: no app bundles to read.
#[cfg(not(target_os = "macos"))]
#[must_use]
pub fn app_with_bundle_id(_id: &str) -> Option<std::path::PathBuf> {
    None
}

/// Off macOS: no bundle ids.
#[cfg(not(target_os = "macos"))]
#[must_use]
pub fn bundle_id(_path: &std::path::Path) -> Option<String> {
    None
}

/// Off macOS: no icons; the UI draws its letters.
#[cfg(not(target_os = "macos"))]
#[must_use]
pub fn icon_png(_bundle: &std::path::Path, _px: u32) -> Option<Vec<u8>> {
    None
}
