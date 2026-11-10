//! Windows OCR on its own thread, so reading a title never holds up frame
//! capture. The engine sends frames with the area to read and polls for
//! the lines.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::time::{Duration, Instant};

use iu_capture::RgbImage;
use iu_ocr::{Ocr, Rect};

/// What a read is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// The play screen's song info box.
    PlayInfo,
    /// The result screen's title and artist.
    ResultTitle,
}

/// What to read: the frame and its areas, each with its enlargement.
type Job = (Purpose, RgbImage, Vec<(Rect, u32)>);

enum Reply {
    /// Sent once at start: whether Japanese OCR could be created.
    Ready(Result<(), String>),
    Lines(Purpose, Result<Vec<String>, String>),
}

pub struct OcrWorker {
    jobs: Sender<Job>,
    replies: Receiver<Reply>,
    available: Option<bool>,
    busy: bool,
}

impl OcrWorker {
    pub fn spawn() -> Self {
        let (jobs, job_rx) = std::sync::mpsc::channel::<Job>();
        let (reply_tx, replies) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("ocr".into())
            .spawn(move || {
                let ocr = Ocr::new();
                let ready = ocr.as_ref().map(|_| ()).map_err(|e| e.to_string());
                if reply_tx.send(Reply::Ready(ready)).is_err() {
                    return;
                }
                for (purpose, frame, areas) in job_rx {
                    let lines = match &ocr {
                        Ok(o) => read_all(o, &frame, &areas),
                        Err(e) => Err(e.to_string()),
                    };
                    if reply_tx.send(Reply::Lines(purpose, lines)).is_err() {
                        return;
                    }
                }
            })
            .expect("spawn OCR thread");
        Self {
            jobs,
            replies,
            available: None,
            busy: false,
        }
    }

    /// `Some(false)` once known that Japanese OCR is missing.
    pub fn available(&self) -> Option<bool> {
        self.available
    }

    /// Starts reading `rect` of `frame` unless a read is in progress or OCR
    /// is unavailable. Returns whether it started.
    pub fn request(&mut self, purpose: Purpose, frame: &RgbImage, areas: Vec<(Rect, u32)>) -> bool {
        if self.busy || self.available == Some(false) {
            return false;
        }
        self.busy = self.jobs.send((purpose, frame.clone(), areas)).is_ok();
        self.busy
    }

    /// A finished read, if any.
    pub fn poll(&mut self) -> Option<(Purpose, Vec<String>)> {
        loop {
            match self.replies.try_recv() {
                Ok(reply) => {
                    if let Some(done) = self.handle(reply) {
                        return Some(done);
                    }
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return None,
            }
        }
    }

    /// Reads `rect` now, waiting up to `timeout` (a read in progress is
    /// finished and dropped first).
    pub fn read_now(
        &mut self,
        frame: &RgbImage,
        rect: Rect,
        timeout: Duration,
    ) -> Option<Vec<String>> {
        let deadline = Instant::now() + timeout;
        while self.busy || self.available.is_none() {
            let left = deadline.checked_duration_since(Instant::now())?;
            match self.replies.recv_timeout(left) {
                Ok(reply) => {
                    self.handle(reply);
                }
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return None,
            }
        }
        if !self.request(Purpose::ResultTitle, frame, vec![(rect, 1)]) {
            return None;
        }
        loop {
            let left = deadline.checked_duration_since(Instant::now())?;
            match self.replies.recv_timeout(left) {
                Ok(reply) => {
                    if let Some((Purpose::ResultTitle, lines)) = self.handle(reply) {
                        return Some(lines);
                    }
                }
                Err(_) => return None,
            }
        }
    }

    fn handle(&mut self, reply: Reply) -> Option<(Purpose, Vec<String>)> {
        match reply {
            Reply::Ready(Ok(())) => {
                self.available = Some(true);
                None
            }
            Reply::Ready(Err(e)) => {
                tracing::warn!("text recognition unavailable: {e}");
                self.available = Some(false);
                None
            }
            Reply::Lines(purpose, lines) => {
                self.busy = false;
                match lines {
                    Ok(lines) => Some((purpose, lines)),
                    Err(e) => {
                        tracing::warn!("text recognition failed: {e}");
                        None
                    }
                }
            }
        }
    }
}

/// Every area read (each at its scale), lines combined without duplicates.
fn read_all(ocr: &Ocr, frame: &RgbImage, areas: &[(Rect, u32)]) -> Result<Vec<String>, String> {
    let mut lines = Vec::new();
    for &(rect, scale) in areas {
        for line in ocr
            .read_scaled(frame, rect, scale)
            .map_err(|e| e.to_string())?
        {
            if !lines.contains(&line) {
                lines.push(line);
            }
        }
    }
    Ok(lines)
}
