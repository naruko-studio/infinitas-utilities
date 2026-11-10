//! Text recognition with Windows' built-in OCR (`Windows.Media.Ocr`).
//!
//! Used to read song titles and artists: unlike masks learned per song, it
//! also reads songs never seen before. Japanese recognition must be
//! installed (it is on Japanese Windows; elsewhere, add Japanese in
//! Settings → Time & language → Language). English is used as well when
//! present, since the Japanese engine misreads some short Latin titles.
//!
//! The engine objects are created and used on one thread.

use image::RgbImage;
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::core::HSTRING;

#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    #[error("Japanese text recognition (OCR) is not installed in Windows")]
    NoJapanese,
    #[error("the region is outside the image")]
    Region,
    #[error("Windows OCR failed: {0}")]
    Windows(#[from] windows::core::Error),
}

/// A rectangle of the frame to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Where titles are read, per resolution (only 1920×1080 so far).
pub mod areas {
    use iu_core::{PlayLayout, Side};

    use super::Rect;

    /// Result screen: the title and artist lines under the score panels
    /// (the same for both sides and SP/DP).
    pub fn result_title(width: u32, height: u32) -> Option<Rect> {
        ((width, height) == (1920, 1080)).then_some(Rect {
            x: 560,
            y: 958,
            width: 800,
            height: 70,
        })
    }

    /// Play screen: the whole top band, at normal size. Every layout's info
    /// box is in it; at this size OCR reads some titles the enlarged box
    /// misses (`3y3s`), so both are read.
    pub fn play_band(width: u32, height: u32) -> Option<Rect> {
        ((width, height) == (1920, 1080)).then_some(Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 150,
        })
    }

    /// How much [`play_info`] is enlarged before reading.
    pub const PLAY_INFO_SCALE: u32 = 4;

    /// Play screen: the title and artist lines of the song info box, per
    /// layout (measured with the `ocr` example: centred above the BGA on
    /// SP, left-aligned in the info column on DP). Read enlarged: short
    /// titles are missed at their real size.
    pub fn play_info(width: u32, height: u32, layout: PlayLayout) -> Option<Rect> {
        if (width, height) != (1920, 1080) {
            return None;
        }
        let centred = |centre: u32, top: u32| Rect {
            x: centre - SP_INFO_WIDTH / 2,
            y: top,
            width: SP_INFO_WIDTH,
            height: 80,
        };
        Some(match layout {
            PlayLayout::Sp {
                side: Side::P1,
                graph_inner: true,
            } => centred(1440, 58),
            PlayLayout::Sp {
                side: Side::P1,
                graph_inner: false,
            } => centred(1024, 48),
            PlayLayout::Sp {
                side: Side::P2,
                graph_inner: true,
            } => centred(480, 40),
            PlayLayout::Sp {
                side: Side::P2,
                graph_inner: false,
            } => centred(898, 48),
            PlayLayout::Dp { info_left: true } => Rect {
                x: 0,
                y: 66,
                width: 480,
                height: 80,
            },
            PlayLayout::Dp { info_left: false } => Rect {
                x: 1530,
                y: 44,
                width: 390,
                height: 80,
            },
        })
    }

    /// Width read around an SP title (the BGA panel's width).
    const SP_INFO_WIDTH: u32 = 640;
}

pub struct Ocr {
    /// Japanese first, then English if installed.
    engines: Vec<OcrEngine>,
}

impl Ocr {
    /// Fails with [`OcrError::NoJapanese`] when Japanese is not installed.
    pub fn new() -> Result<Self, OcrError> {
        let ja = engine("ja")?.ok_or(OcrError::NoJapanese)?;
        let mut engines = vec![ja];
        engines.extend(engine("en-US")?);
        Ok(Self { engines })
    }

    /// The text lines found in `rect`, from every engine (duplicates
    /// removed), top to bottom per engine.
    pub fn read_lines(&self, img: &RgbImage, rect: Rect) -> Result<Vec<String>, OcrError> {
        Ok(self
            .read_placed(img, rect, 1)?
            .into_iter()
            .map(|l| l.text)
            .collect())
    }

    /// Like [`Ocr::read_lines`], with `rect` enlarged `scale` times first
    /// (smoothly): small and very short text (a one- or two-letter title)
    /// is often missed at its real size.
    pub fn read_scaled(
        &self,
        img: &RgbImage,
        rect: Rect,
        scale: u32,
    ) -> Result<Vec<String>, OcrError> {
        Ok(self
            .read_placed(img, rect, scale)?
            .into_iter()
            .map(|l| l.text)
            .collect())
    }

    /// Lines with where they are on the screen (for measuring layouts).
    pub fn read_placed(
        &self,
        img: &RgbImage,
        rect: Rect,
        scale: u32,
    ) -> Result<Vec<PlacedLine>, OcrError> {
        if rect.x + rect.width > img.width() || rect.y + rect.height > img.height() {
            return Err(OcrError::Region);
        }
        let scale = scale.max(1);
        let crop =
            image::imageops::crop_imm(img, rect.x, rect.y, rect.width, rect.height).to_image();
        let crop = if scale == 1 {
            crop
        } else {
            image::imageops::resize(
                &crop,
                rect.width * scale,
                rect.height * scale,
                image::imageops::FilterType::CatmullRom,
            )
        };
        let bitmap = bitmap(&crop)?;
        let mut lines: Vec<PlacedLine> = Vec::new();
        for engine in &self.engines {
            let result = engine.RecognizeAsync(&bitmap)?.join()?;
            let found = result.Lines()?;
            for i in 0..found.Size()? {
                let line = found.GetAt(i)?;
                let text = line.Text()?.to_string_lossy();
                if text.trim().is_empty() || lines.iter().any(|l| l.text == text) {
                    continue;
                }
                let words = line.Words()?;
                let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, 0f32, 0f32);
                for j in 0..words.Size()? {
                    let r = words.GetAt(j)?.BoundingRect()?;
                    x0 = x0.min(r.X);
                    y0 = y0.min(r.Y);
                    x1 = x1.max(r.X + r.Width);
                    y1 = y1.max(r.Y + r.Height);
                }
                let s = scale as f32;
                lines.push(PlacedLine {
                    text,
                    x: rect.x as f32 + x0 / s,
                    y: rect.y as f32 + y0 / s,
                    width: (x1 - x0) / s,
                    height: (y1 - y0) / s,
                });
            }
        }
        Ok(lines)
    }
}

/// A line of text and its box in screen pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedLine {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

fn engine(tag: &str) -> Result<Option<OcrEngine>, OcrError> {
    let language = Language::CreateLanguage(&HSTRING::from(tag))?;
    if !OcrEngine::IsLanguageSupported(&language)? {
        return Ok(None);
    }
    Ok(OcrEngine::TryCreateFromLanguage(&language).ok())
}

/// The image as a BGRA software bitmap.
fn bitmap(img: &RgbImage) -> Result<SoftwareBitmap, OcrError> {
    let bgra: Vec<u8> = img
        .pixels()
        .flat_map(|p| {
            let [r, g, b] = p.0;
            [b, g, r, 255]
        })
        .collect();
    let writer = DataWriter::new()?;
    writer.WriteBytes(&bgra)?;
    let buffer = writer.DetachBuffer()?;
    let w = i32::try_from(img.width()).map_err(|_| OcrError::Region)?;
    let h = i32::try_from(img.height()).map_err(|_| OcrError::Region)?;
    Ok(SoftwareBitmap::CreateCopyFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        w,
        h,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_outside_the_image_is_refused() {
        let Ok(ocr) = Ocr::new() else {
            eprintln!("no Japanese OCR on this machine; skipped");
            return;
        };
        let img = RgbImage::new(10, 10);
        let r = Rect {
            x: 5,
            y: 0,
            width: 10,
            height: 5,
        };
        assert!(matches!(ocr.read_lines(&img, r), Err(OcrError::Region)));
    }
}
