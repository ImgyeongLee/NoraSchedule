//! D-Day cover images.
//!
//! Uploads are re-encoded on import (EXIF-rotated, scaled down, saved as JPEG) so a
//! 10 MB phone photo becomes a ~150 KB file. Files live in `<data dir>/images` and
//! are deleted as soon as no D-Day references them.

use std::collections::HashSet;
use std::fmt;
use std::io::Cursor;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader, RgbImage};

/// Longest side after import. Cards are ~300 px wide, so this is sharp on retina screens.
pub const MAX_SIDE: u32 = 1280;
pub const JPEG_QUALITY: u8 = 82;
/// Refuse absurdly large uploads before decoding them.
pub const MAX_UPLOAD_BYTES: usize = 40 * 1024 * 1024;

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

/// Decodes any supported image and returns an optimized JPEG.
pub fn optimize(bytes: &[u8]) -> Result<Vec<u8>, ImageError> {
    if bytes.len() > MAX_UPLOAD_BYTES {
        return Err(ImageError::TooLarge);
    }
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut decoder = reader.into_decoder().map_err(|_| ImageError::Unsupported)?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(decoder).map_err(|_| ImageError::Unsupported)?;
    img.apply_orientation(orientation);

    if img.width() > MAX_SIDE || img.height() > MAX_SIDE {
        // `resize` keeps the aspect ratio and fits inside the box.
        img = img.resize(MAX_SIDE, MAX_SIDE, FilterType::CatmullRom);
    }

    let rgb = flatten_on_white(img);
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode_image(&rgb)
        .map_err(|_| ImageError::Unsupported)?;
    Ok(out)
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
        && name.ends_with(".jpg")
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
        && !name.contains("..")
}

/// Writes the optimized image and returns its new file name.
pub fn store(dir: &Path, jpeg: &[u8]) -> std::io::Result<String> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    std::fs::create_dir_all(dir)?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let name = format!("{nanos:x}-{:x}.jpg", COUNTER.fetch_add(1, Ordering::Relaxed));
    std::fs::write(dir.join(&name), jpeg)?;
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
        let output = optimize(&input).unwrap();
        let decoded = image::load_from_memory(&output).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (MAX_SIDE, 853), "keeps the aspect ratio");
        assert!(output.len() * 4 < input.len(), "{} -> {} bytes", input.len(), output.len());
    }

    #[test]
    fn small_images_keep_their_size_and_transparency_becomes_white() {
        let output = optimize(&png(40, 30, 0)).unwrap();
        let decoded = image::load_from_memory(&output).unwrap().to_rgb8();
        assert_eq!(decoded.dimensions(), (40, 30));
        assert!(decoded.get_pixel(5, 5).0.iter().all(|&c| c > 245));
    }

    #[test]
    fn rejects_non_images() {
        assert!(matches!(optimize(b"definitely not an image"), Err(ImageError::Unsupported)));
    }

    #[test]
    fn garbage_collection_keeps_referenced_files_only() {
        let dir = std::env::temp_dir().join(format!("nora_img_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = store(&dir, b"a").unwrap();
        let b = store(&dir, b"b").unwrap();
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
        assert!(!is_safe_name("x.png"));
    }
}
