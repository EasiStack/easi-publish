//! Optional image pre-processing (behind the `image` feature).
//!
//! Embedding images at display resolution rather than at source resolution is
//! the single biggest lever for image-heavy documents. Typst embeds an image at
//! its source resolution regardless of the display size, so an oversized source
//! inflates both the first render (a one-time decode) and every PDF's size.
//!
//! Run your images through [`downscale_image`] (once, e.g. at startup
//! or as a build step) before embedding them. This helper (the `image` feature) 
//! decodes PNG/JPEG and re-encodes PNG, which is all most templates need for 
//! pre-downscaling. 
//! 
//! Note that the embedding Typst itself does is broader and got wider in 0.15.
//! Its PDF image pipeline now also handles JPEG2000 (JPXDecode) and 
//! JBIG2 (JBIG2Decode) sources and blend modes, and parses non-compliant files 
//! more robustly. So a template can `image("scan.jp2")` even though this 
//! pre-processor doesn't decode that format; reach for [`downscale_image`] only 
//! for the oversized-raster case.

use std::io::Cursor;

use crate::error::{PublishError, Result};

/// Decode `bytes`, downscale so neither side exceeds `max_dimension` (preserving
/// aspect ratio with a high-quality Lanczos3 filter), and re-encode as PNG.
///
/// If the image already fits within `max_dimension` on both sides, the original
/// `bytes` are returned unchanged, no needless re-encode.
///
/// Pick `max_dimension` for the display size: a logo shown at 4 cm needs only
/// about 470–940 px even at print DPI, not thousands.
///
/// # Errors
/// Returns [`PublishError::Image`] if the input can't be decoded or the output can't
/// be encoded.
pub fn downscale_image(bytes: &[u8], max_dimension: u32) -> Result<Vec<u8>> {
    let img = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| PublishError::Image(e.to_string()))?
        .decode()
        .map_err(|e| PublishError::Image(e.to_string()))?;

    if img.width() <= max_dimension && img.height() <= max_dimension {
        return Ok(bytes.to_vec());
    }

    let resized = img.resize(
        max_dimension,
        max_dimension,
        image::imageops::FilterType::Lanczos3,
    );

    let mut out = Vec::new();
    resized
        .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| PublishError::Image(e.to_string()))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_bytes(w: u32, h: u32) -> Vec<u8> {
        let img = image::DynamicImage::ImageRgb8(image::RgbImage::new(w, h));
        let mut bytes = Vec::new();
        img.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    fn dimensions(bytes: &[u8]) -> (u32, u32) {
        image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .unwrap()
            .into_dimensions()
            .unwrap()
    }

    #[test]
    fn downscales_oversized_image_preserving_aspect() {
        let src = png_bytes(2000, 1000);
        let out = downscale_image(&src, 500).unwrap();
        let (w, h) = dimensions(&out);
        assert!(w <= 500 && h <= 500, "got {w}x{h}");
        // Aspect ratio (2:1) preserved.
        assert_eq!(w, 500);
        assert_eq!(h, 250);
        assert!(out.len() < src.len());
    }

    #[test]
    fn returns_small_image_unchanged() {
        let src = png_bytes(300, 200);
        let out = downscale_image(&src, 500).unwrap();
        assert_eq!(out, src, "already within bounds -> unchanged bytes");
    }

    #[test]
    fn rejects_non_image_bytes() {
        assert!(matches!(
            downscale_image(b"not an image", 500),
            Err(PublishError::Image(_))
        ));
    }
}
