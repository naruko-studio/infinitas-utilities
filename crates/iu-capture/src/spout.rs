//! Spout2 receiver, implemented directly on Win32 shared memory and D3D11.
//!
//! Spout's protocol (studied from the Spout2 SDK, BSD-2-Clause; no code
//! copied):
//! - Shared memory `SpoutSenderNames` holds the running senders' names in
//!   256-byte, NUL-terminated slots.
//! - Shared memory named after each sender holds a `SharedTextureInfo`:
//!   a 32-bit D3D11 shared-texture handle, width, height, DXGI format, …
//! - The named mutex `<sender>_SpoutAccessMutex` guards texture access.
//! - The named semaphore `<sender>_Count_Semaphore` is released once per
//!   frame by the sender, so its count tells whether a new frame arrived.
//!
//! Doing this ourselves avoids shipping `SpoutLibrary.dll` and calling a
//! C++ vtable over FFI.

use std::ffi::CString;

use image::RgbImage;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HMODULE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAP_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_B8G8R8X8_UNORM,
    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC,
};
use windows::Win32::System::Memory::{
    FILE_MAP_READ, MEMORY_BASIC_INFORMATION, MapViewOfFile, OpenFileMappingA, UnmapViewOfFile,
    VirtualQuery,
};
use windows::Win32::System::Threading::{
    OpenMutexW, OpenSemaphoreW, ReleaseMutex, ReleaseSemaphore, SEMAPHORE_MODIFY_STATE,
    SYNCHRONIZATION_SYNCHRONIZE, WaitForSingleObject,
};
use windows::core::{HSTRING, PCSTR};

use crate::{CaptureError, FrameSource};

const SENDER_NAMES_MAP: &str = "SpoutSenderNames";
const NAME_SLOT_LEN: usize = 256;
/// `SharedTextureInfo`: 5 × u32, 256-byte description, u32 partner id.
const SENDER_INFO_LEN: usize = 5 * 4 + 256 + 4;
/// How long to wait for the sender to finish writing a frame (Spout uses 67 ms).
const ACCESS_TIMEOUT_MS: u32 = 67;

/// What a sender publishes about its shared texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SenderInfo {
    pub share_handle: u32,
    pub width: u32,
    pub height: u32,
    /// DXGI format number.
    pub format: u32,
}

impl SenderInfo {
    /// Parses the first bytes of a `SharedTextureInfo` block.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let field = |i: usize| {
            let b = bytes.get(i * 4..i * 4 + 4)?;
            Some(u32::from_le_bytes(b.try_into().ok()?))
        };
        Some(Self {
            share_handle: field(0)?,
            width: field(1)?,
            height: field(2)?,
            format: field(3)?,
        })
    }
}

/// Splits the `SpoutSenderNames` block into names; stops at the first empty slot.
pub fn parse_sender_names(bytes: &[u8]) -> Vec<String> {
    bytes
        .as_chunks::<NAME_SLOT_LEN>()
        .0
        .iter()
        .map(|slot| {
            let end = slot.iter().position(|&b| b == 0).unwrap_or(slot.len());
            String::from_utf8_lossy(&slot[..end]).into_owned()
        })
        .take_while(|name| !name.is_empty())
        .collect()
}

/// Names of the Spout senders running right now.
pub fn list_senders() -> Vec<String> {
    read_shared_memory(SENDER_NAMES_MAP, None)
        .map(|bytes| parse_sender_names(&bytes))
        .unwrap_or_default()
}

/// The sender's current texture info, if it is running.
pub fn sender_info(name: &str) -> Option<SenderInfo> {
    read_shared_memory(name, Some(SENDER_INFO_LEN)).and_then(|b| SenderInfo::parse(&b))
}

fn c_name(name: &str) -> Option<CString> {
    CString::new(name).ok()
}

/// Copies a named shared-memory block. `len: None` copies the whole mapped region.
fn read_shared_memory(name: &str, len: Option<usize>) -> Option<Vec<u8>> {
    let name = c_name(name)?;
    // SAFETY: `name` is a valid NUL-terminated string that outlives the call.
    let mapping =
        unsafe { OpenFileMappingA(FILE_MAP_READ.0, false, PCSTR(name.as_ptr().cast())) }.ok()?;
    // SAFETY: `mapping` is a valid file-mapping handle opened above.
    let view = unsafe { MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, 0) };
    let result = if view.Value.is_null() {
        None
    } else {
        let mut info = MEMORY_BASIC_INFORMATION::default();
        // SAFETY: `view.Value` is a live mapped view; `info` is a valid out buffer.
        let queried = unsafe {
            VirtualQuery(
                Some(view.Value),
                &mut info,
                size_of::<MEMORY_BASIC_INFORMATION>(),
            )
        };
        let region = if queried == 0 { 0 } else { info.RegionSize };
        let len = len.map_or(region, |l| l.min(region));
        // SAFETY: the view is readable for `region` bytes and `len <= region`.
        let bytes = unsafe { std::slice::from_raw_parts(view.Value.cast::<u8>(), len) }.to_vec();
        // SAFETY: `view` was returned by MapViewOfFile and is unmapped once.
        let _ = unsafe { UnmapViewOfFile(view) };
        Some(bytes)
    };
    // SAFETY: `mapping` is owned here and closed once.
    let _ = unsafe { CloseHandle(mapping) };
    result
}

/// An owned Win32 handle, closed on drop.
struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: the handle is owned by this wrapper and closed exactly once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// The texture we have open for the current sender state.
struct OpenTexture {
    info: SenderInfo,
    shared: ID3D11Texture2D,
    staging: ID3D11Texture2D,
}

/// Receives frames from one named Spout sender.
pub struct SpoutReceiver {
    sender: String,
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    texture: Option<OpenTexture>,
    access_mutex: Option<OwnedHandle>,
    frame_counter: Option<OwnedHandle>,
    last_frame: Option<i32>,
}

impl SpoutReceiver {
    pub fn new(sender: impl Into<String>) -> Result<Self, CaptureError> {
        let mut device = None;
        let mut context = None;
        // SAFETY: all out-pointers are valid; no adapter or software module is passed.
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )?;
        }
        Ok(Self {
            sender: sender.into(),
            device: device.expect("D3D11CreateDevice succeeded without a device"),
            context: context.expect("D3D11CreateDevice succeeded without a context"),
            texture: None,
            access_mutex: None,
            frame_counter: None,
            last_frame: None,
        })
    }

    pub fn sender(&self) -> &str {
        &self.sender
    }

    /// Size of the sender's frames, once connected.
    pub fn frame_size(&self) -> Option<(u32, u32)> {
        self.texture.as_ref().map(|t| (t.info.width, t.info.height))
    }

    /// Drops the open texture and sync objects; the next call reconnects.
    pub fn reset(&mut self) {
        self.texture = None;
        self.access_mutex = None;
        self.frame_counter = None;
        self.last_frame = None;
    }

    fn connect(&mut self, info: SenderInfo) -> Result<(), CaptureError> {
        let format = DXGI_FORMAT(info.format as i32);
        if ![
            DXGI_FORMAT_B8G8R8A8_UNORM,
            DXGI_FORMAT_B8G8R8X8_UNORM,
            DXGI_FORMAT_R8G8B8A8_UNORM,
        ]
        .contains(&format)
        {
            return Err(CaptureError::UnsupportedFormat(info.format));
        }
        let mut shared: Option<ID3D11Texture2D> = None;
        // SAFETY: the handle comes from the sender's published info; D3D11
        // validates it and fails cleanly if it is stale.
        unsafe {
            self.device
                .OpenSharedResource(HANDLE(info.share_handle as usize as *mut _), &mut shared)?
        };
        let shared = shared.expect("OpenSharedResource succeeded without a texture");
        let desc = D3D11_TEXTURE2D_DESC {
            Width: info.width,
            Height: info.height,
            MipLevels: 1,
            ArraySize: 1,
            Format: format,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0,
        };
        let mut staging = None;
        // SAFETY: `desc` is fully initialised and `staging` is a valid out-pointer.
        unsafe {
            self.device
                .CreateTexture2D(&desc, None, Some(&mut staging))?
        };
        self.texture = Some(OpenTexture {
            info,
            shared,
            staging: staging.expect("CreateTexture2D succeeded without a texture"),
        });

        let mutex_name = HSTRING::from(format!("{}_SpoutAccessMutex", self.sender));
        let count_name = HSTRING::from(format!("{}_Count_Semaphore", self.sender));
        // SAFETY: the names are valid wide strings that outlive the calls.
        self.access_mutex = unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, &mutex_name) }
            .ok()
            .map(OwnedHandle);
        // SAFETY: as above.
        self.frame_counter = unsafe {
            OpenSemaphoreW(
                SYNCHRONIZATION_SYNCHRONIZE | SEMAPHORE_MODIFY_STATE,
                false,
                &count_name,
            )
        }
        .ok()
        .map(OwnedHandle);
        self.last_frame = None;
        tracing::info!(
            sender = %self.sender,
            width = info.width,
            height = info.height,
            format = info.format,
            frame_counter = self.frame_counter.is_some(),
            "connected to Spout sender"
        );
        Ok(())
    }

    /// Reads the sender's frame count without changing it.
    /// `None` when the sender does not publish one (then every call is "new").
    fn sender_frame(&self) -> Option<i32> {
        let sem = self.frame_counter.as_ref()?;
        let mut previous = 0;
        // SAFETY: `sem` is an open semaphore with MODIFY_STATE; releasing one
        // and immediately waiting it back leaves the count unchanged.
        unsafe {
            ReleaseSemaphore(sem.0, 1, Some(&mut previous)).ok()?;
            WaitForSingleObject(sem.0, 0);
        }
        Some(previous)
    }

    /// Copies the shared texture into the staging texture under the access mutex.
    fn copy_frame(&self, texture: &OpenTexture) {
        let locked = self.access_mutex.as_ref().is_some_and(|m| {
            // SAFETY: `m` is an open mutex handle with SYNCHRONIZE access.
            let r = unsafe { WaitForSingleObject(m.0, ACCESS_TIMEOUT_MS) };
            r == WAIT_OBJECT_0 || r == WAIT_ABANDONED
        });
        // SAFETY: both textures were created on `self.device` with identical
        // size and format.
        unsafe { self.context.CopyResource(&texture.staging, &texture.shared) };
        if locked && let Some(m) = &self.access_mutex {
            // SAFETY: we own the mutex after the successful wait above.
            let _ = unsafe { ReleaseMutex(m.0) };
        }
    }

    fn read_staging(&self, texture: &OpenTexture) -> Result<RgbImage, CaptureError> {
        let (w, h) = (texture.info.width as usize, texture.info.height as usize);
        let bgr = DXGI_FORMAT(texture.info.format as i32) != DXGI_FORMAT_R8G8B8A8_UNORM;
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        // SAFETY: the staging texture has CPU read access; `mapped` is a valid out buffer.
        unsafe {
            self.context
                .Map(&texture.staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?
        };
        let pitch = mapped.RowPitch as usize;
        // SAFETY: while mapped, pData points to `pitch * h` readable bytes.
        let src = unsafe { std::slice::from_raw_parts(mapped.pData.cast::<u8>(), pitch * h) };
        let mut rgb = Vec::with_capacity(w * h * 3);
        for row in src.chunks_exact(pitch) {
            for px in row[..w * 4].as_chunks::<4>().0 {
                if bgr {
                    rgb.extend_from_slice(&[px[2], px[1], px[0]]);
                } else {
                    rgb.extend_from_slice(&px[..3]);
                }
            }
        }
        // SAFETY: the texture was mapped above and is unmapped once.
        unsafe { self.context.Unmap(&texture.staging, 0) };
        Ok(RgbImage::from_raw(w as u32, h as u32, rgb).expect("buffer matches the frame size"))
    }
}

impl FrameSource for SpoutReceiver {
    fn next_frame(&mut self) -> Result<Option<RgbImage>, CaptureError> {
        let Some(info) = sender_info(&self.sender) else {
            self.reset();
            return Err(CaptureError::SenderNotFound(self.sender.clone()));
        };
        // A new handle or size means OBS recreated the texture.
        if self.texture.as_ref().is_none_or(|t| t.info != info) {
            self.reset();
            self.connect(info)?;
        }

        if let Some(frame) = self.sender_frame() {
            if self.last_frame == Some(frame) {
                return Ok(None);
            }
            self.last_frame = Some(frame);
        }

        let texture = self.texture.as_ref().expect("connected above");
        self.copy_frame(texture);
        self.read_staging(texture).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sender_names_until_first_empty_slot() {
        let mut block = vec![0u8; NAME_SLOT_LEN * 4];
        block[..16].copy_from_slice(b"Spout_OBS_Filter");
        block[NAME_SLOT_LEN..NAME_SLOT_LEN + 5].copy_from_slice(b"Other");
        // slot 2 empty, slot 3 stale: must not be listed
        block[NAME_SLOT_LEN * 3..NAME_SLOT_LEN * 3 + 5].copy_from_slice(b"Stale");
        assert_eq!(parse_sender_names(&block), ["Spout_OBS_Filter", "Other"]);
    }

    #[test]
    fn parses_sender_info() {
        let mut block = vec![0u8; SENDER_INFO_LEN];
        for (i, v) in [0x4000_1A42u32, 1920, 1080, 87, 0].iter().enumerate() {
            block[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        assert_eq!(
            SenderInfo::parse(&block),
            Some(SenderInfo {
                share_handle: 0x4000_1A42,
                width: 1920,
                height: 1080,
                format: 87,
            })
        );
        assert_eq!(SenderInfo::parse(&block[..8]), None);
    }

    #[test]
    fn missing_sender_is_reported() {
        let mut receiver = SpoutReceiver::new("iu-test-sender-that-does-not-exist").unwrap();
        assert!(matches!(
            receiver.next_frame(),
            Err(CaptureError::SenderNotFound(_))
        ));
    }
}
