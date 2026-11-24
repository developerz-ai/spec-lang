// Projected by hand from type Asset, asset_name (src/hop/render.slg) and load_assets (docs/contract.md section 4). Stand-in for spec-lang build output; change the spec, not this file.

use std::ffi::CString;
use std::fmt;
use std::path::{Path, PathBuf};

use super::ffi;
use crate::render::{Asset, asset_name};

/// The lit shader and one model per `Asset`, loaded after the window opened and unloaded in reverse before it closes.
#[derive(Debug)]
pub struct Assets {
    pub(super) lit: ffi::Shader,
    pub(super) time_loc: i32,
    models: Vec<ffi::Model>,
}

/// `error: asset <path>: cannot load`, exit 3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetError {
    pub path: PathBuf,
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "asset {}: cannot load", self.path.display())
    }
}

fn c_path(path: &Path) -> Result<CString, AssetError> {
    use std::os::unix::ffi::OsStrExt;
    CString::new(path.as_os_str().as_bytes()).map_err(|_| AssetError {
        path: path.to_owned(),
    })
}

impl Assets {
    /// `load_assets()`. Needs an open window (a GL context): `Window::load_assets` is the only caller.
    pub(super) fn load(root: &Path) -> Result<Self, AssetError> {
        let vs = root.join("shaders/lit.vs");
        let fs = root.join("shaders/lit.fs");
        let (vs_c, fs_c) = (c_path(&vs)?, c_path(&fs)?);
        // SAFETY: NUL-terminated paths that outlive the call.
        let lit = unsafe { ffi::LoadShader(vs_c.as_ptr(), fs_c.as_ptr()) };
        // raylib falls back to its default shader when a file is missing; that is a failed load too.
        // SAFETY: plain calls on the shader value raylib returned.
        let valid = unsafe { ffi::IsShaderValid(lit) && lit.id != ffi::rlGetShaderIdDefault() };
        if !valid {
            // SAFETY: unloading the default shader is a no-op in raylib; unloading a loaded one frees it.
            unsafe { ffi::UnloadShader(lit) };
            return Err(AssetError { path: vs });
        }
        // SAFETY: a valid shader and a NUL-terminated name.
        let time_loc = unsafe { ffi::GetShaderLocation(lit, c"uTime".as_ptr()) };
        let mut assets = Self {
            lit,
            time_loc,
            models: Vec::with_capacity(Asset::ALL.len()),
        };
        for asset in Asset::ALL {
            let path = root.join(format!("out/{}.glb", asset_name(asset)));
            let file = c_path(&path)?;
            // SAFETY: a NUL-terminated path that outlives the call.
            let model = unsafe { ffi::LoadModel(file.as_ptr()) };
            // SAFETY: a model value raylib returned.
            if !unsafe { ffi::IsModelValid(model) } {
                // SAFETY: `UnloadModel` frees what `LoadModel` allocated, valid or not.
                unsafe { ffi::UnloadModel(model) };
                return Err(AssetError { path });
            }
            let count = usize::try_from(model.material_count).unwrap_or(0);
            if !model.materials.is_null() {
                // SAFETY: `materials` points to `materialCount` materials owned by the model, which we hold alone.
                let materials = unsafe { std::slice::from_raw_parts_mut(model.materials, count) };
                for m in materials {
                    m.shader = lit;
                }
            }
            assets.models.push(model);
        }
        Ok(assets)
    }

    pub(super) fn model(&self, asset: Asset) -> ffi::Model {
        self.models[asset.index()]
    }
}

impl Drop for Assets {
    /// Unload in reverse order. A model's materials hold the lit shader; raylib's `UnloadMaterial` skips the default shader
    /// only, so each material's shader is reset to the default first and the lit shader is unloaded once, last.
    fn drop(&mut self) {
        // SAFETY: each model was loaded by `LoadModel` and is unloaded once; the window is still open (`Window` owns `Assets`).
        unsafe {
            let default = ffi::rlGetShaderIdDefault();
            while let Some(model) = self.models.pop() {
                let count = usize::try_from(model.material_count).unwrap_or(0);
                if !model.materials.is_null() {
                    for m in std::slice::from_raw_parts_mut(model.materials, count) {
                        m.shader.id = default;
                    }
                }
                ffi::UnloadModel(model);
            }
            ffi::UnloadShader(self.lit);
        }
    }
}
