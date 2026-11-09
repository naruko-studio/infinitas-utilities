//! Frame capture.
//!
//! The only real source is Spout2 (see [`spout`]). [`FrameSource`] lets
//! tests and the dev tools feed screenshots through the same pipeline.

pub mod spout;

pub use image::RgbImage;

#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    /// No sender with this name is running (OBS closed, filter missing…).
    #[error("Spout sender `{0}` not found")]
    SenderNotFound(String),
    #[error("Spout sender uses texture format {0}, which is not supported")]
    UnsupportedFormat(u32),
    #[error("Windows API call failed: {0}")]
    Windows(#[from] windows::core::Error),
}

/// Something that yields game frames.
pub trait FrameSource {
    /// The next frame, or `None` if nothing new arrived since the last call.
    fn next_frame(&mut self) -> Result<Option<RgbImage>, CaptureError>;
}

/// Yields the same image on every call. For tests and offline tools.
pub struct StillImage(pub RgbImage);

impl FrameSource for StillImage {
    fn next_frame(&mut self) -> Result<Option<RgbImage>, CaptureError> {
        Ok(Some(self.0.clone()))
    }
}
