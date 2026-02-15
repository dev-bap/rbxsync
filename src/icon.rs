use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result};
use image::ImageFormat;

use crate::alpha_bleed;

/// Loads an icon from disk, optionally applies alpha bleed, and returns processed PNG bytes.
pub fn process_icon(path: &Path, bleed: bool) -> Result<Vec<u8>> {
    let mut img =
        image::open(path).with_context(|| format!("Failed to open icon: {}", path.display()))?;

    if bleed {
        alpha_bleed::alpha_bleed(&mut img);
    }

    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .with_context(|| format!("Failed to encode icon: {}", path.display()))?;

    Ok(buf)
}
