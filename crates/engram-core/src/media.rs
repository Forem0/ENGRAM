use crate::{digest, scan::metadata_for, EngramError};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::{
    codecs::png::PngEncoder, ExtendedColorType, ImageEncoder, ImageFormat, ImageReader, Limits,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    io::{self, Cursor, Read, Write},
    path::PathBuf,
};

pub const MAX_MEDIA_INPUT_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_MEDIA_PIXELS: u64 = 16_000_000;
pub const MAX_MEDIA_PREVIEW_EDGE: u32 = 1024;
pub const MAX_MEDIA_OUTPUT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug)]
struct MediaSource {
    path: PathBuf,
    bytes: u64,
    sha256: String,
}

/// An ephemeral authority created by an actual scan, never deserialized from a
/// case or renderer request. Its physical paths never cross the native bridge.
#[derive(Debug)]
pub struct MediaAccess {
    root: PathBuf,
    case_id: String,
    sources: HashMap<String, MediaSource>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MediaPreview {
    pub finding_id: String,
    pub data_url: String,
    pub width: u32,
    pub height: u32,
    pub source_width: u32,
    pub source_height: u32,
    pub source_sha256: String,
}

impl MediaAccess {
    pub(crate) fn new(root: PathBuf, case_id: String) -> Self {
        Self {
            root,
            case_id,
            sources: HashMap::new(),
        }
    }

    pub(crate) fn register(&mut self, id: String, path: PathBuf, bytes: u64, sha256: String) {
        if bytes > 0 && bytes <= MAX_MEDIA_INPUT_BYTES {
            self.sources.insert(
                id,
                MediaSource {
                    path,
                    bytes,
                    sha256,
                },
            );
        }
    }

    pub fn case_id(&self) -> &str {
        &self.case_id
    }

    /// Decode one captured file after checking its current bytes against the
    /// observation. Output contains only newly encoded PNG pixels; source EXIF,
    /// text chunks, filenames and other metadata are not copied into the image.
    pub fn preview(&self, finding_id: &str) -> Result<MediaPreview, EngramError> {
        let source = self.sources.get(finding_id).ok_or_else(|| unavailable(
            "This finding has no readable source captured by the current scan. Select its source and scan again.",
        ))?;
        let before = metadata_for(&self.root, &source.path)?;
        if before.len() != source.bytes {
            return Err(unavailable(
                "The source image changed since this case was scanned. Scan it again.",
            ));
        }
        let file = fs::File::open(&source.path)?;
        let opened = file.metadata()?;
        if !opened.is_file() || opened.len() != source.bytes {
            return Err(unavailable("The source image changed while opening it."));
        }
        let mut bytes = Vec::new();
        file.take(source.bytes + 1).read_to_end(&mut bytes)?;
        let after = metadata_for(&self.root, &source.path)?;
        if bytes.len() as u64 != source.bytes
            || after.len() != source.bytes
            || before.modified().ok() != after.modified().ok()
            || digest(&bytes) != source.sha256
        {
            return Err(unavailable(
                "The source image changed since this case was scanned. Scan it again.",
            ));
        }
        let format = image::guess_format(&bytes)
            .map_err(|_| unavailable("This file is not a supported PNG or JPEG image."))?;
        if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg) {
            return Err(unavailable("Image previews currently support PNG and JPEG. Other media retain their recorded metadata."));
        }
        let mut dimensions_reader = ImageReader::with_format(Cursor::new(&bytes), format);
        dimensions_reader.limits(decode_limits());
        let (source_width, source_height) = dimensions_reader.into_dimensions().map_err(|_| {
            unavailable("The image dimensions are invalid or exceed preview limits.")
        })?;
        if source_width == 0
            || source_height == 0
            || u64::from(source_width) * u64::from(source_height) > MAX_MEDIA_PIXELS
        {
            return Err(unavailable(
                "Image previews are limited to 16 million source pixels.",
            ));
        }
        let mut reader = ImageReader::with_format(Cursor::new(&bytes), format);
        reader.limits(decode_limits());
        let decoded = reader.decode().map_err(|_| {
            unavailable("This image could not be decoded within the preview limits.")
        })?;
        let resized =
            if source_width <= MAX_MEDIA_PREVIEW_EDGE && source_height <= MAX_MEDIA_PREVIEW_EDGE {
                decoded.to_rgba8()
            } else {
                decoded
                    .thumbnail(MAX_MEDIA_PREVIEW_EDGE, MAX_MEDIA_PREVIEW_EDGE)
                    .to_rgba8()
            };
        let (width, height) = resized.dimensions();
        let mut output = BoundedOutput(Vec::new());
        PngEncoder::new(&mut output)
            .write_image(resized.as_raw(), width, height, ExtendedColorType::Rgba8)
            .map_err(|_| {
                unavailable(
                    "The rendered preview exceeds the output limit or could not be encoded.",
                )
            })?;
        Ok(MediaPreview {
            finding_id: finding_id.into(),
            data_url: format!("data:image/png;base64,{}", STANDARD.encode(&output.0)),
            width,
            height,
            source_width,
            source_height,
            source_sha256: source.sha256.clone(),
        })
    }
}

fn unavailable(message: &str) -> EngramError {
    EngramError::Input(message.into())
}

fn decode_limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(16_000);
    limits.max_image_height = Some(16_000);
    limits.max_alloc = Some(128 * 1024 * 1024);
    limits
}

struct BoundedOutput(Vec<u8>);

impl Write for BoundedOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0.len().saturating_add(bytes.len()) > MAX_MEDIA_OUTPUT_BYTES {
            return Err(io::Error::other("Preview output limit reached"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_writer_rejects_overflow_without_truncating_or_appending() {
        let mut output = BoundedOutput(vec![0; MAX_MEDIA_OUTPUT_BYTES - 1]);
        output.write_all(&[1]).unwrap();
        assert!(output.write_all(&[2]).is_err());
        assert_eq!(output.0.len(), MAX_MEDIA_OUTPUT_BYTES);
        assert_eq!(output.0.last(), Some(&1));
    }
}
