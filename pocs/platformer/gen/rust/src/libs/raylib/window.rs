//! The one raylib window of the process, with its assets: what the drawing calls draw on.

use super::assets::Assets;
use super::ffi;
use crate::rt::app::App;
use std::cell::RefCell;
use std::ffi::{CString, c_int};
use std::path::{Path, PathBuf};

thread_local! {
    /// The assets of the window open now.
    static OPEN: RefCell<Option<Assets>> = const { RefCell::new(None) };
}

/// `f` on the open window's assets; `None` when no window is open.
pub fn with<T>(f: impl FnOnce(&Assets) -> T) -> Option<T> {
    OPEN.with(|o| o.borrow().as_ref().map(f))
}

/// What a window is asked for.
#[derive(Debug)]
pub struct Spec<'s> {
    /// The executable's name: `<BIN>_ASSETS` is read.
    pub bin: &'s str,
    pub title: &'s str,
    pub width: c_int,
    pub height: c_int,
    /// Never shown, for frames read back; else on screen, multisampled.
    pub hidden: bool,
    /// Frames per second, for a window on screen.
    pub fps: Option<c_int>,
}

/// raylib 5.5's `InitWindow` does not stop when no display can be opened: it goes on with no GL loaded and crashes. With no
/// display named there is nothing to open, so that is refused before the call.
fn display_named() -> bool {
    ["DISPLAY", "WAYLAND_DISPLAY"].iter().any(|n| std::env::var_os(n).is_some_and(|v| !v.is_empty()))
}

const CANNOT_OPEN: &str = "window: cannot open";

/// An open window. Dropping it unloads the assets, then closes it.
#[derive(Debug)]
pub struct Window(());

pub fn open(app: &App, spec: &Spec<'_>) -> Result<Window, String> {
    if !display_named() {
        return Err(CANNOT_OPEN.into());
    }
    let title = CString::new(spec.title).map_err(|_| CANNOT_OPEN.to_string())?;
    // SAFETY: plain configuration calls, then `InitWindow` with a NUL-terminated title that outlives it.
    let ready = unsafe {
        ffi::SetTraceLogLevel(ffi::LOG_NONE);
        ffi::SetConfigFlags(if spec.hidden { ffi::FLAG_WINDOW_HIDDEN } else { ffi::FLAG_MSAA_4X_HINT });
        ffi::InitWindow(spec.width, spec.height, title.as_ptr());
        ffi::IsWindowReady()
    };
    if !ready {
        return Err(CANNOT_OPEN.into());
    }
    let window = Window(());
    let variable = format!("{}_ASSETS", spec.bin.to_uppercase().replace('-', "_"));
    let root =
        std::env::var_os(variable).filter(|v| !v.is_empty()).map_or_else(|| PathBuf::from("assets"), PathBuf::from);
    let assets = Assets::load(app, &root)?;
    OPEN.with(|o| *o.borrow_mut() = Some(assets));
    if let Some(fps) = spec.fps {
        // SAFETY: the window is open.
        unsafe { ffi::SetTargetFPS(fps) };
    }
    Ok(window)
}

impl Window {
    /// Esc or the close button.
    pub fn closing(&self) -> bool {
        // SAFETY: the window is open.
        unsafe { ffi::WindowShouldClose() }
    }

    /// One frame on screen.
    pub fn frame<T>(&self, draw: impl FnOnce() -> T) -> T {
        // SAFETY: the window is open; the two calls match.
        unsafe { ffi::BeginDrawing() };
        let drawn = draw();
        // SAFETY: as above.
        unsafe { ffi::EndDrawing() };
        drawn
    }

    /// One frame drawn off screen and read back as R8G8B8A8, top row first.
    pub fn capture<T>(&self, width: c_int, height: c_int, draw: impl FnOnce() -> T) -> Result<(T, Picture), String> {
        // SAFETY: the window is open; the render texture is unloaded below.
        let target = unsafe { ffi::LoadRenderTexture(width, height) };
        // SAFETY: a value raylib returned.
        if !unsafe { ffi::IsRenderTextureValid(target) } {
            return Err(CANNOT_OPEN.into());
        }
        // SAFETY: a valid render texture, drawn to between the begin and end calls.
        unsafe { ffi::BeginTextureMode(target) };
        let drawn = draw();
        // SAFETY: as above; then the colour attachment is read back, flipped, and the texture freed.
        let image = unsafe {
            ffi::EndTextureMode();
            let mut image = ffi::LoadImageFromTexture(target.texture);
            ffi::UnloadRenderTexture(target);
            ffi::ImageFlipVertical(&raw mut image);
            image
        };
        Ok((drawn, Picture { image }))
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        OPEN.with(|o| o.borrow_mut().take());
        // SAFETY: the window was opened by `open`.
        unsafe { ffi::CloseWindow() };
    }
}

/// A frame read back from the GPU. Freed on drop, before the window closes.
#[derive(Debug)]
pub struct Picture {
    image: ffi::Image,
}

impl Picture {
    /// `width * height * 4` bytes of R8G8B8A8.
    pub fn bytes(&self) -> &[u8] {
        let (w, h) = (self.image.width, self.image.height);
        if self.image.data.is_null() || self.image.format != ffi::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 || w <= 0 || h <= 0
        {
            return &[];
        }
        let len = usize::try_from(w).unwrap_or(0) * usize::try_from(h).unwrap_or(0) * 4;
        // SAFETY: raylib allocated `width * height` pixels of four bytes for this format; the image lives as long as `self`.
        unsafe { std::slice::from_raw_parts(self.image.data.cast::<u8>(), len) }
    }

    /// Writes the image as PNG; `false` when the file cannot be written.
    pub fn export(&self, path: &Path) -> bool {
        use std::os::unix::ffi::OsStrExt;
        let Ok(file) = CString::new(path.as_os_str().as_bytes()) else { return false };
        // SAFETY: a valid image and a NUL-terminated path that outlives the call.
        unsafe { ffi::ExportImage(self.image, file.as_ptr()) }
    }
}

impl Drop for Picture {
    fn drop(&mut self) {
        // SAFETY: the image came from `LoadImageFromTexture` and is freed once.
        unsafe { ffi::UnloadImage(self.image) };
    }
}
