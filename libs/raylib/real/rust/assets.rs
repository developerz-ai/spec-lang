//! The shader and the models a window loads, after it opens: `<assets>/shaders/lit.{vs,fs}` (its uniform `uTime` is
//! `BeginShaderMode`'s argument) and one model per variant of the sum the program's `raylib#DrawModelEx` takes first, in
//! declared order, from `<assets>/out/<name>.glb`: the name is what the program's one `fn` from that sum to Text gives,
//! or the variant's name in lower case when there is no such `fn`.

use super::ffi;
use crate::machine::Machine;
use crate::rt::app::{App, Func};
use crate::rt::data::{Data, Shape, Ty};
use std::ffi::CString;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Assets {
    pub lit: ffi::Shader,
    pub time_loc: i32,
    /// By the variant's name, in load order.
    models: Vec<(&'static str, ffi::Model)>,
}

fn cannot_load(path: &Path) -> String {
    format!("asset {}: cannot load", path.display())
}

fn c_path(path: &Path) -> Result<CString, String> {
    use std::os::unix::ffi::OsStrExt;
    CString::new(path.as_os_str().as_bytes()).map_err(|_| cannot_load(path))
}

/// The model sum's variants and their files' names.
fn names(app: &App) -> Result<Vec<(&'static str, String)>, String> {
    let draw = app.externs.iter().find(|x| x.library == "raylib" && x.symbol == "DrawModelEx");
    let Some(sum) = draw.and_then(|x| x.params.first()).map(|p| p.ty) else { return Ok(Vec::new()) };
    let Shape::Sum(variants) = &sum.shape else { return Ok(Vec::new()) };
    let takes =
        |f: &&Func| matches!(f.params, [p] if std::ptr::eq::<Ty>(p.ty, sum)) && matches!(f.result.shape, Shape::Text);
    let mut namers = app.fns.iter().filter(takes);
    let namer = match (namers.next(), namers.next()) {
        (Some(f), None) => Some(f),
        _ => None,
    };
    let mut machine = Machine::new().map_err(|h| h.message())?;
    let mut out = Vec::new();
    for (variant, _) in *variants {
        let name = match namer {
            Some(f) => match (f.call)(&mut machine, vec![Some(Data::Variant(variant, Vec::new()))]) {
                Ok(Data::Text(t)) => t,
                Ok(_) => return Err(format!("{} gave no Text", f.qualified)),
                Err(h) => return Err(h.message()),
            },
            None => variant.to_lowercase(),
        };
        out.push((*variant, name));
    }
    Ok(out)
}

impl Assets {
    /// Needs an open window (a GL context). The message names the file that did not load.
    pub fn load(app: &App, root: &Path) -> Result<Self, String> {
        let names = names(app)?;
        let vs = root.join("shaders/lit.vs");
        let (vs_c, fs_c) = (c_path(&vs)?, c_path(&root.join("shaders/lit.fs"))?);
        // SAFETY: NUL-terminated paths that outlive the call.
        let lit = unsafe { ffi::LoadShader(vs_c.as_ptr(), fs_c.as_ptr()) };
        // raylib falls back to its default shader when a file is missing; that is a failed load too.
        // SAFETY: plain calls on the shader value raylib returned.
        let valid = unsafe { ffi::IsShaderValid(lit) && lit.id != ffi::rlGetShaderIdDefault() };
        if !valid {
            // SAFETY: unloading the default shader is a no-op in raylib; unloading a loaded one frees it.
            unsafe { ffi::UnloadShader(lit) };
            return Err(cannot_load(&vs));
        }
        // SAFETY: a valid shader and a NUL-terminated name.
        let time_loc = unsafe { ffi::GetShaderLocation(lit, c"uTime".as_ptr()) };
        let mut assets = Assets { lit, time_loc, models: Vec::with_capacity(names.len()) };
        for (variant, name) in names {
            let path = root.join(PathBuf::from(format!("out/{name}.glb")));
            let file = c_path(&path)?;
            // SAFETY: a NUL-terminated path that outlives the call.
            let model = unsafe { ffi::LoadModel(file.as_ptr()) };
            // SAFETY: a model value raylib returned.
            if !unsafe { ffi::IsModelValid(model) } {
                // SAFETY: `UnloadModel` frees what `LoadModel` allocated, valid or not.
                unsafe { ffi::UnloadModel(model) };
                return Err(cannot_load(&path));
            }
            let count = usize::try_from(model.material_count).unwrap_or(0);
            if !model.materials.is_null() {
                // raylib draws a model with its materials' shader; BeginShaderMode does not reach DrawModelEx.
                // SAFETY: `materials` points to `materialCount` materials owned by the model, which we hold alone.
                for m in unsafe { std::slice::from_raw_parts_mut(model.materials, count) } {
                    m.shader = lit;
                }
            }
            assets.models.push((variant, model));
        }
        Ok(assets)
    }

    /// A model by the variant that names it, or by its index in load order.
    pub fn model(&self, key: &Data) -> Option<ffi::Model> {
        match key {
            Data::Variant(name, _) => self.models.iter().find(|(n, _)| n == name).map(|(_, m)| *m),
            Data::Int(i) => usize::try_from(*i).ok().and_then(|i| self.models.get(i)).map(|(_, m)| *m),
            _ => None,
        }
    }
}

impl Drop for Assets {
    /// In reverse order. A model's materials hold the lit shader; raylib's `UnloadMaterial` skips the default shader
    /// only, so each material's shader is reset to the default first and the lit shader is unloaded once, last.
    fn drop(&mut self) {
        // SAFETY: each model was loaded by `LoadModel` and is unloaded once; the window is still open (it owns these).
        unsafe {
            let default = ffi::rlGetShaderIdDefault();
            while let Some((_, model)) = self.models.pop() {
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
