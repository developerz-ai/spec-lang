// Projected by hand from note @window(960, 540, "Skyhop") (src/apps/rust.slg) and the shot and play sequences of contract section 2. Stand-in for spec-lang build output; change the spec, not this file.

use std::ffi::{CStr, CString};
use std::fmt;
use std::path::Path;

use super::assets::{AssetError, Assets};
use super::ffi;
use super::present::present;
use crate::world::World;

const TITLE: &CStr = c"Skyhop";
/// `@window(960, 540, "Skyhop")`.
pub const PLAY_SIZE: (i32, i32) = (960, 540);
/// The off-screen frame of `shot`.
pub const SHOT_SIZE: (i32, i32) = (320, 180);

/// What stops a window from being used: exit 3 either way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowError {
    /// `error: window: cannot open`
    Open,
    Asset(AssetError),
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => f.write_str("window: cannot open"),
            Self::Asset(e) => e.fmt(f),
        }
    }
}

/// raylib 5.5's `InitWindow` does not stop when GLFW cannot start: it goes on to `rlglInit` with no GL loaded and
/// crashes, so `IsWindowReady` is never reached. With no display named there is nothing GLFW could open, so that case is
/// refused before the call. A display that is named but cannot be reached still crashes inside raylib (NOTES.md, Limits).
fn display_named() -> bool {
    ["DISPLAY", "WAYLAND_DISPLAY"]
        .iter()
        .any(|name| std::env::var_os(name).is_some_and(|v| !v.is_empty()))
}

/// An open raylib window with the assets loaded. raylib has one window per process; dropping this unloads the assets,
/// then closes it.
#[derive(Debug)]
pub struct Window {
    assets: Option<Assets>,
}

impl Window {
    fn open(width: i32, height: i32, flags: u32, assets: &Path) -> Result<Self, WindowError> {
        if !display_named() {
            return Err(WindowError::Open);
        }
        // SAFETY: plain configuration calls, then `InitWindow` with a NUL-terminated title.
        let ready = unsafe {
            ffi::SetTraceLogLevel(ffi::LOG_NONE);
            ffi::SetConfigFlags(flags);
            ffi::InitWindow(width, height, TITLE.as_ptr());
            ffi::IsWindowReady()
        };
        if !ready {
            return Err(WindowError::Open);
        }
        let mut window = Self { assets: None };
        window.assets = Some(Assets::load(assets).map_err(WindowError::Asset)?);
        Ok(window)
    }

    /// The hidden 320 x 180 window of `shot`.
    pub fn hidden(assets: &Path) -> Result<Self, WindowError> {
        Self::open(SHOT_SIZE.0, SHOT_SIZE.1, ffi::FLAG_WINDOW_HIDDEN, assets)
    }

    /// The window of `play`, at 60 frames per second.
    pub fn visible(assets: &Path) -> Result<Self, WindowError> {
        let window = Self::open(PLAY_SIZE.0, PLAY_SIZE.1, ffi::FLAG_MSAA_4X_HINT, assets)?;
        // SAFETY: the window is open.
        unsafe { ffi::SetTargetFPS(60) };
        Ok(window)
    }

    /// Esc or the close button.
    pub fn should_close(&self) -> bool {
        // SAFETY: the window is open.
        unsafe { ffi::WindowShouldClose() }
    }

    /// One frame on screen: `BeginDrawing(); present(); EndDrawing();`.
    pub fn frame(&self, world: &World) {
        let Some(assets) = &self.assets else { return };
        // SAFETY: the window is open.
        unsafe { ffi::BeginDrawing() };
        present(world, assets);
        // SAFETY: matches the `BeginDrawing` above.
        unsafe { ffi::EndDrawing() };
    }

    /// Draws the frame off screen and reads it back as R8G8B8A8, top row first.
    pub fn capture(&self, world: &World) -> Result<Capture, WindowError> {
        let Some(assets) = &self.assets else {
            return Err(WindowError::Open);
        };
        // SAFETY: the window is open; the render texture is unloaded below on every path.
        let rt = unsafe { ffi::LoadRenderTexture(SHOT_SIZE.0, SHOT_SIZE.1) };
        // SAFETY: a value raylib returned.
        if !unsafe { ffi::IsRenderTextureValid(rt) } {
            return Err(WindowError::Open);
        }
        // SAFETY: a valid render texture, drawn to between the begin and end calls.
        unsafe { ffi::BeginTextureMode(rt) };
        present(world, assets);
        // SAFETY: as above; then the colour attachment is read back and the texture freed.
        let image = unsafe {
            ffi::EndTextureMode();
            let mut image = ffi::LoadImageFromTexture(rt.texture);
            ffi::UnloadRenderTexture(rt);
            // Top row first, as the window shows it (contract section 2).
            ffi::ImageFlipVertical(&raw mut image);
            image
        };
        Ok(Capture { image })
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        self.assets = None;
        // SAFETY: the window was opened by `open` (an unopened one returned early and was never built).
        unsafe { ffi::CloseWindow() };
    }
}

/// A frame read back from the GPU. Freed on drop.
#[derive(Debug)]
pub struct Capture {
    image: ffi::Image,
}

impl Capture {
    /// The image bytes: `width * height * 4` of R8G8B8A8, as contract section 2 hashes them.
    pub fn bytes(&self) -> &[u8] {
        let (w, h) = (self.image.width, self.image.height);
        if self.image.data.is_null()
            || self.image.format != ffi::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8
            || w <= 0
            || h <= 0
        {
            return &[];
        }
        let len = usize::try_from(w).unwrap_or(0) * usize::try_from(h).unwrap_or(0) * 4;
        // SAFETY: raylib allocated `width * height` pixels of four bytes for this format; the image lives as long as `self`.
        unsafe { std::slice::from_raw_parts(self.image.data.cast::<u8>(), len) }
    }

    /// `ExportImage` as PNG; `false` when raylib could not write the file.
    pub fn export(&self, path: &Path) -> bool {
        use std::os::unix::ffi::OsStrExt;
        let Ok(file) = CString::new(path.as_os_str().as_bytes()) else {
            return false;
        };
        // SAFETY: a valid image and a NUL-terminated path that outlives the call.
        unsafe { ffi::ExportImage(self.image, file.as_ptr()) }
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        // SAFETY: the image came from `LoadImageFromTexture` and is freed once.
        unsafe { ffi::UnloadImage(self.image) };
    }
}
