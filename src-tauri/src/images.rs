//! User images: D-Day covers, the Overview header and image cards, and stickers.
//!
//! Uploads are re-encoded on import (EXIF-rotated, scaled down, saved as JPEG) so a
//! 10 MB phone photo becomes a ~150 KB file. Stickers with transparency are kept as PNG,
//! and GIFs are kept unchanged so they stay animated.
//! Files live in `<data dir>/images` and are deleted as soon as nothing references them.

use std::collections::HashSet;
use std::fmt;
use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageEncoder, ImageReader, RgbImage};
use serde::Deserialize;

/// Longest side after import. Cards are ~300 px wide, so this is sharp on retina screens.
pub const MAX_SIDE: u32 = 1280;

/// What an upload is for; decides how large it is kept and whether transparency survives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    /// D-Day covers.
    #[default]
    Cover,
    /// Decorative Overview image cards.
    Card,
    /// The wide Overview header, shown across the whole page.
    Header,
    /// Decorations; transparent images stay PNG.
    Sticker,
}

impl Purpose {
    fn max_side(self) -> u32 {
        match self {
            Purpose::Cover => MAX_SIDE,
            Purpose::Card => 1600,
            Purpose::Header => 2400,
            Purpose::Sticker => 640,
        }
    }
}
pub const JPEG_QUALITY: u8 = 82;
/// Refuse absurdly large uploads before decoding them.
pub const MAX_UPLOAD_BYTES: usize = 40 * 1024 * 1024;
/// Animated GIFs are kept unchanged, so they get a tighter limit.
pub const MAX_GIF_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug)]
pub enum ImageError {
    TooLarge,
    Unsupported,
    Io(std::io::Error),
}

impl fmt::Display for ImageError {
    // These codes are translated by the frontend.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::TooLarge => write!(f, "image_too_large"),
            ImageError::Unsupported => write!(f, "image_unsupported"),
            ImageError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl From<std::io::Error> for ImageError {
    fn from(e: std::io::Error) -> Self {
        ImageError::Io(e)
    }
}

/// Decodes any supported image and returns the optimized file and its extension
/// (`jpg`, `png` for stickers with transparency, or `gif`).
pub fn optimize(bytes: &[u8], purpose: Purpose) -> Result<(Vec<u8>, &'static str), ImageError> {
    if bytes.len() > MAX_UPLOAD_BYTES {
        return Err(ImageError::TooLarge);
    }
    // Re-encoding would keep only the first frame, so GIFs (any purpose) are stored as they are
    // (after checking they really are GIFs) and keep their animation.
    if image::guess_format(bytes).ok() == Some(image::ImageFormat::Gif) {
        if bytes.len() > MAX_GIF_BYTES {
            return Err(ImageError::TooLarge);
        }
        image::load_from_memory_with_format(bytes, image::ImageFormat::Gif).map_err(|_| ImageError::Unsupported)?;
        return Ok((bytes.to_vec(), "gif"));
    }
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut decoder = reader.into_decoder().map_err(|_| ImageError::Unsupported)?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder).map_err(|_| ImageError::Unsupported)?;
    img.apply_orientation(orientation);

    let max = purpose.max_side();
    if img.width() > max || img.height() > max {
        // `resize` keeps the aspect ratio and fits inside the box.
        img = img.resize(max, max, FilterType::CatmullRom);
    }

    // Only stickers that actually have see-through pixels need PNG; the rest are smaller as JPEG.
    if purpose == Purpose::Sticker && img.color().has_alpha() && img.to_rgba8().pixels().any(|p| p.0[3] < 255) {
        let rgba = img.to_rgba8();
        let mut out = Vec::new();
        PngEncoder::new(&mut out)
            .write_image(rgba.as_raw(), rgba.width(), rgba.height(), image::ExtendedColorType::Rgba8)
            .map_err(|_| ImageError::Unsupported)?;
        return Ok((out, "png"));
    }

    let rgb = flatten_on_white(img);
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode_image(&rgb)
        .map_err(|_| ImageError::Unsupported)?;
    Ok((out, "jpg"))
}

/// JPEG has no transparency; composite transparent pixels over white.
fn flatten_on_white(img: DynamicImage) -> RgbImage {
    if !img.color().has_alpha() {
        return img.to_rgb8();
    }
    let rgba = img.to_rgba8();
    RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let p = rgba.get_pixel(x, y).0;
        let a = p[3] as u32;
        let blend = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
        image::Rgb([blend(p[0]), blend(p[1]), blend(p[2])])
    })
}

/// Only names this module generates are ever read or deleted.
pub fn is_safe_name(name: &str) -> bool {
    name.len() <= 64
        && [".jpg", ".png", ".gif"].iter().any(|ext| name.ends_with(ext))
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
        && !name.contains("..")
}

/// MIME type for a stored image name.
pub fn content_type(name: &str) -> &'static str {
    if name.ends_with(".png") {
        "image/png"
    } else if name.ends_with(".gif") {
        "image/gif"
    } else {
        "image/jpeg"
    }
}

/// Writes the optimized image and returns its new file name.
pub fn store(dir: &Path, bytes: &[u8], ext: &str) -> std::io::Result<String> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    std::fs::create_dir_all(dir)?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let name = format!("{nanos:x}-{:x}.{ext}", COUNTER.fetch_add(1, Ordering::Relaxed));
    std::fs::write(dir.join(&name), bytes)?;
    Ok(name)
}

/// Deletes every stored image that is not in `keep`. Returns how many were removed.
pub fn collect_garbage(dir: &Path, keep: &HashSet<String>) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else { return 0 };
    let mut removed = 0;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_safe_name(&name) && !keep.contains(&name) && std::fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgba, RgbaImage};

    fn png(width: u32, height: u32, alpha: u8) -> Vec<u8> {
        // A noisy gradient so the PNG is realistically large.
        let img = RgbaImage::from_fn(width, height, |x, y| {
            Rgba([(x * 7 % 256) as u8, (y * 13 % 256) as u8, ((x ^ y) % 256) as u8, alpha])
        });
        let mut out = Cursor::new(Vec::new());
        img.write_to(&mut out, ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn large_images_are_scaled_down_and_shrunk() {
        let input = png(3000, 2000, 255);
        let (output, ext) = optimize(&input, Purpose::Cover).unwrap();
        assert_eq!(ext, "jpg");
        let decoded = image::load_from_memory(&output).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (MAX_SIDE, 853), "keeps the aspect ratio");
        assert!(output.len() * 4 < input.len(), "{} -> {} bytes", input.len(), output.len());
    }

    #[test]
    fn small_images_keep_their_size_and_transparency_becomes_white() {
        let (output, _) = optimize(&png(40, 30, 0), Purpose::Cover).unwrap();
        let decoded = image::load_from_memory(&output).unwrap().to_rgb8();
        assert_eq!(decoded.dimensions(), (40, 30));
        assert!(decoded.get_pixel(5, 5).0.iter().all(|&c| c > 245));
    }

    #[test]
    fn rejects_non_images() {
        assert!(matches!(optimize(b"definitely not an image", Purpose::Cover), Err(ImageError::Unsupported)));
    }

    #[test]
    fn transparent_stickers_stay_png_and_keep_their_alpha() {
        let (output, ext) = optimize(&png(2000, 1000, 0), Purpose::Sticker).unwrap();
        assert_eq!(ext, "png");
        let decoded = image::load_from_memory(&output).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (640, 320));
        assert_eq!(decoded.get_pixel(5, 5).0[3], 0);
        // Opaque stickers are fine as JPEG.
        assert_eq!(optimize(&png(40, 30, 255), Purpose::Sticker).unwrap().1, "jpg");
    }

    fn gif(frames: u32) -> Vec<u8> {
        use image::codecs::gif::GifEncoder;
        use image::{Delay, Frame};
        let mut out = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut out);
            for i in 0..frames {
                let img = RgbaImage::from_pixel(20, 10, Rgba([(i * 80) as u8, 0, 0, 255]));
                encoder.encode_frame(Frame::from_parts(img, 0, 0, Delay::from_numer_denom_ms(100, 1))).unwrap();
            }
        }
        out
    }

    #[test]
    fn gifs_are_kept_as_is_so_they_stay_animated() {
        let input = gif(3);
        let (output, ext) = optimize(&input, Purpose::Sticker).unwrap();
        assert_eq!(ext, "gif");
        assert_eq!(output, input);
        assert_eq!(optimize(&input, Purpose::Header).unwrap().1, "gif");
        assert_eq!(optimize(&input, Purpose::Card).unwrap().1, "gif");
        assert_eq!(optimize(&input, Purpose::Cover).unwrap().1, "gif");
        // Something that only claims to be a GIF is refused.
        assert!(matches!(optimize(b"GIF89a broken", Purpose::Sticker), Err(ImageError::Unsupported)));
    }

    #[test]
    fn garbage_collection_keeps_referenced_files_only() {
        let dir = std::env::temp_dir().join(format!("nora_img_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = store(&dir, b"a", "jpg").unwrap();
        let b = store(&dir, b"b", "png").unwrap();
        std::fs::write(dir.join("notes.txt"), "not ours").unwrap();
        assert_eq!(collect_garbage(&dir, &HashSet::from([a.clone()])), 1);
        assert!(dir.join(&a).exists());
        assert!(!dir.join(&b).exists());
        assert!(dir.join("notes.txt").exists(), "files we did not create are left alone");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unsafe_names_are_rejected() {
        assert!(is_safe_name("18f3a-1.jpg"));
        assert!(!is_safe_name("../nora.sqlite3"));
        assert!(!is_safe_name("a/b.jpg"));
        assert!(is_safe_name("18f3a-2.png"));
        assert!(is_safe_name("18f3a-3.gif"));
        assert!(!is_safe_name("x.bmp"));
    }
}
