//! An app's project: the program, the manifest as Rust data with calls from plain data (`src/manifest.rs`), and
//! the runtimes of the libraries the app uses, copied from `libs/<name>/real/rust/` into `src/libs/<name>/`.
//! A library's `runtime.conf` says what the printer needs to know of it, as data.

use crate::code::Fx;
use crate::cx::{Cx, RECORD};
use crate::dty::{conform_fn, describe, snake};
use crate::names::{camel, ident};
use spec_lang_diag::Diag;
use spec_lang_lir::manifest::{Manifest, Named, Note, NoteArg, NoteValue};
use spec_lang_lir::{DTy, Extern, LTy, TypeDef};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A foreign body in an app: a call into the library's adapter, its arguments and result as plain data.
pub fn extern_call(fx: &mut Fx<'_, '_>, x: &Extern, result: &LTy, params: &[String]) -> Vec<String> {
    let args: Vec<String> = params.iter().map(|p| format!("rt::data::AsData::to_data(&{p})")).collect();
    let ty = fx.ty(result);
    vec![
        format!(
            "let reply = crate::libs::{}::call({:?}, &[{}])?;",
            ident(&x.library),
            x.symbol,
            args.join(", ")
        ),
        format!(
            "<{ty} as rt::data::AsData>::from_data(&reply).ok_or_else(|| rt::stop(\"E-TYPE-MISMATCH\", \"`{}#{}` gave a value of another type\"))",
            x.library, x.symbol
        ),
    ]
}

/// A library's runtime for Rust, as found on disk.
#[derive(Debug)]
pub struct Runtime {
    pub name: String,
    pub dir: PathBuf,
    pub conf: BTreeMap<String, String>,
}

fn read_conf(dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Ok(text) = std::fs::read_to_string(dir.join("runtime.conf")) {
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            if let Some((k, v)) = line.split_once('=') {
                out.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    out
}

/// Where library `name`'s Rust runtime is: beside the loaded library of that name, else beside any loaded one.
fn find_runtime(cx: &Cx<'_>, name: &str) -> Option<Runtime> {
    let mut dirs: Vec<PathBuf> = cx.lir.libraries.iter().filter(|l| l.name == name).map(|l| PathBuf::from(&l.dir)).collect();
    dirs.extend(cx.lir.libraries.iter().filter_map(|l| Path::new(&l.dir).parent().map(|p| p.join(name))));
    dirs.into_iter().map(|d| d.join("real").join("rust")).find(|d| d.is_dir()).map(|dir| Runtime {
        name: name.to_string(),
        conf: read_conf(&dir),
        dir,
    })
}

/// The libraries an app needs a runtime of, in first-use order: its shape, its notes' and triggers', its
/// handlers', the ones its foreign bodies name, and what their `runtime.conf` says they need.
pub fn libraries(cx: &Cx<'_>, m: &Manifest) -> Result<Vec<Runtime>, Diag> {
    let lir = cx.lir;
    let mut names: Vec<String> = Vec::new();
    let mut add = |n: &str| {
        if !n.is_empty() && !names.iter().any(|x| x == n) {
            names.push(n.to_string());
        }
    };
    if let Some(s) = &m.shape {
        add(s);
    }
    for n in &m.notes {
        add(&n.library);
    }
    for r in &m.rules {
        for t in &lir.rule(*r).triggers {
            add(&t.library);
        }
        if let spec_lang_lir::Body::Extern(x) = &lir.rule(*r).body {
            add(&x.library);
        }
    }
    for f in &lir.fns {
        if let spec_lang_lir::Body::Extern(x) = &f.body {
            add(&x.library);
        }
    }
    let mut out: Vec<Runtime> = Vec::new();
    let mut i = 0;
    while i < names.len() {
        let name = names[i].clone();
        i += 1;
        let Some(rt) = find_runtime(cx, &name) else {
            return Err(Diag::load(format!(
                "E-UNSUPPORTED: app {} uses library `{name}`, which has no Rust runtime (libs/{name}/real/rust/)",
                m.qualified
            )));
        };
        for need in rt.conf.get("needs").map(|s| s.split_whitespace().map(str::to_string).collect::<Vec<_>>()).unwrap_or_default() {
            if !names.contains(&need) {
                names.push(need);
            }
        }
        out.push(rt);
    }
    Ok(out)
}

/// The library whose `main` runs the app: its shape's, else the one `runtime.conf` marks `entry = true`.
pub fn entry<'r>(m: &Manifest, libs: &'r [Runtime]) -> Option<&'r Runtime> {
    if let Some(s) = &m.shape {
        return libs.iter().find(|l| &l.name == s);
    }
    libs.iter().find(|l| l.conf.get("entry").is_some_and(|v| v == "true"))
}

/// The executable's name: `executable = <note>.<param>` of the entry's `runtime.conf`, else the app's name.
pub fn executable(m: &Manifest, entry: Option<&Runtime>) -> String {
    let named = entry.and_then(|e| e.conf.get("executable")).and_then(|spec| {
        let (note, param) = spec.split_once('.')?;
        let n = m.notes.iter().find(|n| n.name == note)?;
        n.args.iter().find(|a| a.param == param).and_then(|a| match &a.value {
            NoteValue::Text(t) => Some(t.clone()),
            _ => None,
        })
    });
    named.unwrap_or_else(|| snake(&m.name))
}

/// Every file of a library runtime's directory, relative to it.
pub fn runtime_files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![PathBuf::new()];
    while let Some(rel) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(dir.join(&rel)) else { continue };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let r = rel.join(e.file_name());
            if e.path().is_dir() {
                stack.push(r);
            } else if let Ok(bytes) = std::fs::read(e.path()) {
                out.push((r.to_string_lossy().replace('\\', "/"), bytes));
            }
        }
    }
    out.sort();
    out
}

// ---- the manifest as Rust ----

/// Statics describing declared types: `TY_<word>`, by DTy.
#[derive(Debug, Default)]
struct Tys {
    items: Vec<(DTy, String)>,
}

impl Tys {
    fn name(&mut self, cx: &Cx<'_>, d: &DTy) -> String {
        if let Some((_, n)) = self.items.iter().find(|(x, _)| x == d) {
            return n.clone();
        }
        let base = format!("TY_{}", word(cx, d).to_uppercase());
        let mut n = base.clone();
        let mut k = 2;
        while self.items.iter().any(|(_, x)| *x == n) {
            n = format!("{base}_{k}");
            k += 1;
        }
        self.items.push((d.clone(), n.clone()));
        n
    }
}

fn word(cx: &Cx<'_>, d: &DTy) -> String {
    match d {
        DTy::Named(t) => snake(&cx.lir.ty(*t).info.name),
        DTy::Option(x) => format!("option_{}", word(cx, x)),
        DTy::List(x) => format!("list_{}", word(cx, x)),
        DTy::Set(x) => format!("set_{}", word(cx, x)),
        DTy::Map(k, v) => format!("map_{}_{}", word(cx, k), word(cx, v)),
        DTy::Tuple(xs) => format!("tuple_{}", xs.iter().map(|x| word(cx, x)).collect::<Vec<_>>().join("_")),
        DTy::Range { lo, hi, .. } => format!("range_{}_{}", lo.to_string().replace('-', "m"), hi.to_string().replace('-', "m")),
        DTy::Where(b, c) => format!("{}_where{}", word(cx, b), c.index()),
        other => format!("{other:?}").to_lowercase(),
    }
}

/// A declared type for a value that has only an `LTy` (a result without a written check).
fn dty_of(t: &LTy) -> DTy {
    match t {
        LTy::Bool => DTy::Bool,
        LTy::Int => DTy::Int,
        LTy::Text => DTy::Text,
        LTy::Named(id) => DTy::Named(*id),
        LTy::Option(x) => DTy::Option(Box::new(dty_of(x))),
        LTy::List(x) => DTy::List(Box::new(dty_of(x))),
        LTy::Set(x) => DTy::Set(Box::new(dty_of(x))),
        LTy::Map(k, v) => DTy::Map(Box::new(dty_of(k)), Box::new(dty_of(v))),
        LTy::Tuple(xs) => DTy::Tuple(xs.iter().map(dty_of).collect()),
        _ => DTy::Unit,
    }
}

fn ty_items(cx: &mut Cx<'_>, tys: &mut Tys) -> Vec<String> {
    let lir = cx.lir;
    let mut out = Vec::new();
    let mut i = 0;
    while i < tys.items.len() {
        let (d, name) = tys.items[i].clone();
        i += 1;
        let shown = match &d {
            DTy::Named(t) => lir.ty(*t).info.name.clone(),
            _ => describe(cx, &d),
        };
        let shape = shape(cx, tys, &d);
        let rust = cx.ty(&cx.lty(&d), None);
        let check = if cx.trivial(&d) {
            "|_| true".to_string()
        } else {
            let f = conform_fn(cx, &d);
            if cx.dty_m(&d) {
                format!("|v| Machine::new().and_then(|mut m| crate::conform::{f}(&mut m, v)).unwrap_or(false)")
            } else {
                format!("|v| crate::conform::{f}(v).unwrap_or(false)")
            }
        };
        let opaque = matches!(shape.as_str(), "Shape::Opaque");
        let admits = if opaque { "|_| false".to_string() } else { format!("|d| rt::data::admits::<{rust}>(d, {check})") };
        out.push(format!("/// `{}`.\npub static {name}: Ty = Ty {{ name: {shown:?}, shape: {shape}, admits: {admits} }};", describe(cx, &d)));
    }
    out
}

fn shape(cx: &Cx<'_>, tys: &mut Tys, d: &DTy) -> String {
    let lir = cx.lir;
    let mut r = |x: &DTy| format!("&{}", tys.name(cx, x));
    match d {
        DTy::Unit => "Shape::Unit".into(),
        DTy::Bool => "Shape::Bool".into(),
        DTy::Int | DTy::Range { .. } => "Shape::Int".into(),
        DTy::Text => "Shape::Text".into(),
        DTy::Option(x) => format!("Shape::Option({})", r(x)),
        DTy::List(x) => format!("Shape::List({})", r(x)),
        DTy::Set(x) => format!("Shape::Set({})", r(x)),
        DTy::Map(k, v) => {
            let k = r(k);
            format!("Shape::Map({k}, {})", r(v))
        }
        DTy::Tuple(xs) => format!("Shape::Tuple(&[{}])", xs.iter().map(&mut r).collect::<Vec<_>>().join(", ")),
        DTy::Where(b, _) => shape(cx, tys, b),
        DTy::Trace(_) => "Shape::Opaque".into(),
        DTy::Named(t) => {
            let decl = lir.ty(*t);
            match &decl.def {
                TypeDef::Alias { of } => shape(cx, tys, of),
                TypeDef::Record { fields } => {
                    let fs: Vec<String> = fields.iter().map(|f| format!("({:?}, {})", f.name, r(&f.check))).collect();
                    format!("Shape::Record(&[{}])", fs.join(", "))
                }
                TypeDef::Sum { variants } if !decl.builtin => {
                    let vs: Vec<String> = variants
                        .iter()
                        .map(|v| {
                            let fs: Vec<String> = v.fields.iter().map(|f| format!("({:?}, {})", f.name, r(&f.check))).collect();
                            format!("({:?}, &[{}])", v.name, fs.join(", "))
                        })
                        .collect();
                    format!("Shape::Sum(&[{}])", vs.join(", "))
                }
                _ => "Shape::Opaque".into(),
            }
        }
    }
}

/// `AsData` for every record and sum of the program, and the world.
fn as_data_items(cx: &Cx<'_>, names: &[String]) -> Vec<String> {
    let lir = cx.lir;
    let mut out = Vec::new();
    for (i, decl) in lir.types.iter().enumerate() {
        if decl.builtin {
            continue;
        }
        let id = spec_lang_ir::TypeId(i as u32);
        let path = cx.type_path(id, None);
        match &decl.def {
            TypeDef::Record { fields } => {
                let to: Vec<String> = fields.iter().map(|f| format!("({:?}, self.{}.to_data())", f.name, ident(&f.name))).collect();
                let from: Vec<String> =
                    fields.iter().map(|f| format!("{}: AsData::from_data(d.field({:?})?)?", ident(&f.name), f.name)).collect();
                out.push(format!(
                    "impl AsData for {path} {{ fn to_data(&self) -> Data {{ Data::Record(vec![{}]) }} fn from_data(d: &Data) -> Option<Self> {{ if !matches!(d, Data::Record(_)) {{ return None; }} Some({path} {{ {} }}) }} }}",
                    to.join(", "),
                    from.join(", ")
                ));
                let _ = RECORD;
            }
            TypeDef::Sum { variants } => {
                let mut to = Vec::new();
                let mut from = Vec::new();
                for v in variants {
                    let vn = ident(&camel(&v.name));
                    let fs: Vec<String> = v.fields.iter().map(|f| ident(&f.name)).collect();
                    let pairs: Vec<String> = v.fields.iter().map(|f| format!("({:?}, {}.to_data())", f.name, ident(&f.name))).collect();
                    let inits: Vec<String> =
                        v.fields.iter().map(|f| format!("{}: AsData::from_data(d.field({:?})?)?", ident(&f.name), f.name)).collect();
                    if v.fields.is_empty() {
                        to.push(format!("{path}::{vn} => Data::Variant({:?}, Vec::new()),", v.name));
                        from.push(format!("{:?} => Some({path}::{vn}),", v.name));
                    } else {
                        to.push(format!("{path}::{vn} {{ {} }} => Data::Variant({:?}, vec![{}]),", fs.join(", "), v.name, pairs.join(", ")));
                        from.push(format!("{:?} => Some({path}::{vn} {{ {} }}),", v.name, inits.join(", ")));
                    }
                }
                out.push(format!(
                    "impl AsData for {path} {{ fn to_data(&self) -> Data {{ match self {{ {} }} }} fn from_data(d: &Data) -> Option<Self> {{ let Data::Variant(name, _) = d else {{ return None }}; match *name {{ {} _ => None }} }} }}",
                    to.join(" "),
                    from.join(" ")
                ));
            }
            _ => {}
        }
    }
    let to: Vec<String> = lir
        .states
        .iter()
        .enumerate()
        .map(|(i, _)| format!("({:?}, w.{}.to_data())", names[i], cx.names.state(spec_lang_ir::StateId(i as u32))))
        .collect();
    let from: Vec<String> = lir
        .states
        .iter()
        .enumerate()
        .map(|(i, _)| format!("{}: AsData::from_data(d.field({:?})?)?", cx.names.state(spec_lang_ir::StateId(i as u32)), names[i]))
        .collect();
    out.push(format!("/// The world as data: every state by name.\nfn world_data(w: &World) -> Data {{ Data::Record(vec![{}]) }}", to.join(", ")));
    out.push(format!(
        "/// A world from data, when every state is of its Rust type (claims are checked apart).\nfn data_world(d: &Data) -> Option<World> {{ Some(World {{ {} }}) }}",
        from.join(", ")
    ));
    out
}

fn note_text(n: &Note) -> String {
    format!("Note {{ library: {:?}, name: {:?}, args: &[{}] }}", n.library, n.name, n.args.iter().map(arg_text).collect::<Vec<_>>().join(", "))
}

fn arg_text(a: &NoteArg) -> String {
    let v = match &a.value {
        NoteValue::Int(i) => format!("NoteValue::Int({i})"),
        NoteValue::Text(t) => format!("NoteValue::Text({t:?})"),
        NoteValue::Quantity { value, unit } => format!("NoteValue::Quantity({value}, {unit:?})"),
        NoteValue::Variant { name, args } => {
            format!("NoteValue::Variant({name:?}, &[{}])", args.iter().map(arg_text).collect::<Vec<_>>().join(", "))
        }
    };
    format!("NoteArg {{ param: {:?}, value: {v} }}", a.param)
}

/// `src/manifest.rs` for app `m`.
pub fn manifest_file(cx: &mut Cx<'_>, m: &Manifest) -> String {
    let lir = cx.lir;
    let mut tys = Tys::default();
    let mut items = Vec::new();
    let state_names: Vec<String> = lir
        .states
        .iter()
        .map(|s| if lir.states.iter().filter(|x| x.info.name == s.info.name).count() > 1 { s.info.qualified.clone() } else { s.info.name.clone() })
        .collect();

    // Calls from data.
    let glue = |cx: &mut Cx<'_>, tys: &mut Tys, params: &[spec_lang_lir::Param], path: &str, m_arg: bool, f: &str| {
        let mut lines = vec!["let mut args = args.into_iter();".to_string()];
        let mut call = Vec::new();
        if m_arg {
            call.push("m".to_string());
        }
        let mut infos = Vec::new();
        for (i, p) in params.iter().enumerate() {
            let t = cx.ty(&p.ty, None);
            let getter = if p.default.is_some() { "arg_or_default" } else { "arg" };
            lines.push(format!("let a{i} = rt::app::{getter}::<{t}>(&mut args, {:?})?;", p.name));
            call.push(format!("a{i}"));
            infos.push(format!("Param {{ name: {:?}, ty: &{}, has_default: {} }}", p.name, tys.name(cx, &p.check), p.default.is_some()));
        }
        lines.push(format!("Ok({path}({})?.to_data())", call.join(", ")));
        (format!("fn {f}(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {{ {} }}", lines.join(" ")), infos)
    };

    let state_labels: Vec<String> =
        lir.states.iter().flat_map(|s| [format!("{}.write", s.info.name), format!("{}.write", s.info.qualified)]).collect();
    let mut rules = Vec::new();
    let mut rule_index = BTreeMap::new();
    for (k, r) in m.rules.iter().enumerate() {
        rule_index.insert(*r, k);
        let rule = lir.rule(*r);
        let path = cx.names.path(None, rule.info.module, &ident(&rule.info.name));
        let f = format!("call_{}", ident(&snake(&rule.info.name)).trim_start_matches("r#"));
        let f = format!("{f}_{k}");
        let (item, params) = glue(cx, &mut tys, &rule.params, &path, true, &f);
        items.push(item);
        let result = rule.result_check.clone().unwrap_or_else(|| dty_of(&rule.result));
        let result = tys.name(cx, &result);
        let triggers: Vec<String> = rule.triggers.iter().map(note_text).collect();
        let uses: Vec<String> = rule.uses.iter().map(|u| format!("{u:?}")).collect();
        let writes = rule.uses.iter().any(|u| state_labels.contains(u));
        rules.push(format!(
            "Rule {{ name: {:?}, qualified: {:?}, params: &[{}], result: &{result}, triggers: &[{}], uses: &[{}], writes: {writes}, call: {f} }}",
            rule.info.name,
            rule.info.qualified,
            params.join(", "),
            triggers.join(", "),
            uses.join(", ")
        ));
    }
    let mut fns = Vec::new();
    let mut fn_index = BTreeMap::new();
    for p in &m.params {
        for d in &p.decls {
            if let Named::Fn(id) = d {
                if fn_index.contains_key(id) {
                    continue;
                }
                fn_index.insert(*id, fns.len());
                let func = lir.func(*id);
                let path = cx.names.path(None, func.info.module, &ident(&func.info.name));
                let f = format!("fn_{}_{}", ident(&snake(&func.info.name)).trim_start_matches("r#"), fns.len());
                let m_arg = cx.fn_m[id.index()];
                let (item, params) = glue(cx, &mut tys, &func.params, &path, m_arg, &f);
                items.push(item);
                let result = func.result_check.clone().unwrap_or_else(|| dty_of(&func.result));
                let result = tys.name(cx, &result);
                fns.push(format!(
                    "Func {{ name: {:?}, qualified: {:?}, params: &[{}], result: &{result}, call: {f} }}",
                    func.info.name,
                    func.info.qualified,
                    params.join(", ")
                ));
            }
        }
    }
    let params: Vec<String> = m
        .params
        .iter()
        .map(|p| {
            let ds: Vec<String> = p
                .decls
                .iter()
                .map(|d| match d {
                    Named::Rule(r) => format!("Decl::Rule({})", rule_index.get(r).copied().unwrap_or(usize::MAX)),
                    Named::Fn(f) => format!("Decl::Fn({})", fn_index.get(f).copied().unwrap_or(usize::MAX)),
                })
                .collect();
            format!("AppParam {{ name: {:?}, decls: &[{}] }}", p.name, ds.join(", "))
        })
        .collect();
    let states: Vec<String> = lir
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| format!("State {{ name: {:?}, qualified: {:?}, ty: &{} }}", state_names[i], s.info.qualified, tys.name(cx, &s.check)))
        .collect();
    let mut reads = Vec::new();
    for (hi, h) in lir.handlers.iter().enumerate() {
        for (oi, op) in h.ops.iter().enumerate() {
            if op.params.is_some() {
                continue;
            }
            let f = crate::machine::op_fn(lir, spec_lang_ir::HandlerId(hi as u32), oi as u32);
            reads.push(format!(
                "Read {{ library: {:?}, handler: {:?}, op: {:?}, read: |m| Ok(crate::machine::{f}(m)?.to_data()) }}",
                h.library, h.name, op.name
            ));
        }
    }
    items.extend(as_data_items(cx, &state_names));
    items.push(format!(
        "/// The app `{}`: what its libraries' runtimes read.\npub static APP: App = App {{ name: {:?}, qualified: {:?}, module: {:?}, target: {}, shape: {}, notes: &[{}], params: &[{}], rules: &[{}], fns: &[{}], states: &[{}], reads: &[{}], world_data, data_world, lawful: rt::runner::lawful }};",
        m.qualified,
        m.name,
        m.qualified,
        m.module,
        m.target.as_ref().map_or("None".into(), |t| format!("Some({t:?})")),
        m.shape.as_ref().map_or("None".into(), |t| format!("Some({t:?})")),
        m.notes.iter().map(note_text).collect::<Vec<_>>().join(", "),
        params.join(", "),
        rules.join(", "),
        fns.join(", "),
        states.join(", "),
        reads.join(", ")
    ));
    items.extend(ty_items(cx, &mut tys));
    items.join("\n\n")
}
