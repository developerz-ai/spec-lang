//! Types: an `LTy` as a TypeScript type, and an `LTy` or `DTy` as a runtime `Type` value (`rt.int`,
//! `rt.list(Task)`, `TaskNo`), which orders, shows and checks values of it.

use crate::file::TsFile;
use crate::names::{int_literal, string_literal};
use crate::plan::Plan;
use spec_lang_ir::{ModuleId, TypeId};
use spec_lang_lir::program::TypeDef;
use spec_lang_lir::{DTy, LTy};

/// A file being printed, for the module whose names it may use bare.
#[derive(Debug)]
pub struct Out<'a, 'l> {
    pub plan: &'a Plan<'l>,
    pub file: TsFile,
    pub module: Option<ModuleId>,
    /// What a claim could not say; every file reports these with its own.
    pub errors: Vec<spec_lang_diag::Diag>,
}

impl<'a, 'l> Out<'a, 'l> {
    pub fn new(plan: &'a Plan<'l>, file: TsFile, module: Option<ModuleId>) -> Self {
        Out { plan, file, module, errors: Vec::new() }
    }

    pub fn rt(&mut self) -> &'static str {
        self.file.rt()
    }

    /// Imports the runtime when printed text reads it.
    pub fn note_rt(&mut self, text: &str) {
        let reads = text.match_indices("rt.").any(|(i, _)| {
            !text[..i].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '$')
        });
        if reads {
            self.rt();
        }
    }

    pub fn rt_type(&mut self) -> &'static str {
        self.file.rt_type()
    }

    /// A declaration of `module` named in a type.
    pub fn decl_type(&mut self, module: ModuleId, name: &str) -> String {
        if Some(module) == self.module {
            return name.to_string();
        }
        let alias = self.plan.module_alias[module.index()].clone();
        let path = self.plan.module_file[module.index()].clone();
        self.file.namespace_type(&path, &alias);
        format!("{alias}.{name}")
    }

    /// A declaration of `module` named `name` (its TypeScript name), from this file.
    pub fn decl(&mut self, module: ModuleId, name: &str) -> String {
        if Some(module) == self.module {
            return name.to_string();
        }
        let alias = self.plan.module_alias[module.index()].clone();
        let path = self.plan.module_file[module.index()].clone();
        self.file.namespace(&path, &alias);
        format!("{alias}.{name}")
    }

    /// A type declaration by id, from this file.
    pub fn type_ref(&mut self, t: TypeId) -> String {
        let decl = self.plan.lir.ty(t);
        let name = self.plan.type_name[t.index()].clone();
        self.decl(decl.info.module, &name)
    }

    /// The machine file's `World` (the live world) or `WorldValue` (a moment of it), as a type.
    pub fn machine_type(&mut self, name: &str) -> String {
        self.file.type_only("src/machine.ts", name);
        name.to_string()
    }

    /// The machine file's value `name`.
    pub fn machine_value(&mut self, name: &str) -> String {
        self.file.value("src/machine.ts", name);
        name.to_string()
    }

    /// An `LTy` as a TypeScript type.
    pub fn ts_type(&mut self, t: &LTy) -> String {
        match t {
            LTy::Unit => "null".into(),
            LTy::Bool => "boolean".into(),
            LTy::Int => "bigint".into(),
            LTy::Text => "string".into(),
            LTy::Named(id) if *id == self.plan.lir.result_type => format!("{}.Reply", self.rt_type()),
            LTy::Named(id) if *id == self.plan.lir.option_type => format!("{}.Option<unknown>", self.rt_type()),
            LTy::Named(id) => match self.plan.lir.ty(*id).def {
                TypeDef::RuleSet { .. } => format!("{}.Event", self.rt_type()),
                _ => {
                    let decl = self.plan.lir.ty(*id);
                    let name = self.plan.type_name[id.index()].clone();
                    self.decl_type(decl.info.module, &name)
                }
            },
            LTy::Option(x) => format!("{}.Option<{}>", self.rt_type(), self.ts_type(x)),
            LTy::List(x) => readonly_array(&self.ts_type(x)),
            LTy::Set(x) => format!("{}.OrdSet<{}>", self.rt_type(), self.ts_type(x)),
            LTy::Map(k, v) => format!("{}.OrdMap<{}, {}>", self.rt_type(), self.ts_type(k), self.ts_type(v)),
            LTy::Tuple(xs) => {
                let items: Vec<String> = xs.iter().map(|x| self.ts_type(x)).collect();
                format!("readonly [{}]", items.join(", "))
            }
            LTy::Range => format!("{}.Range", self.rt_type()),
            LTy::Func(ps, r) => {
                let params: Vec<String> =
                    ps.iter().enumerate().map(|(i, p)| format!("a{i}: {}", self.ts_type(p))).collect();
                format!("({}) => {}", params.join(", "), self.ts_type(r))
            }
            LTy::Event(_) => format!("{}.Event", self.rt_type()),
            LTy::Reply(_) => format!("{}.Reply", self.rt_type()),
            LTy::Failure => format!("{}.FailureValue", self.rt_type()),
            LTy::Step(_) => {
                let v = self.machine_type("WorldValue");
                format!("{}.Step<{v}>", self.rt_type())
            }
            LTy::Trace(_) => {
                let v = self.machine_type("WorldValue");
                format!("{}.Trace<{v}>", self.rt_type())
            }
            LTy::World => self.machine_type("WorldValue"),
            LTy::Never => "never".into(),
            LTy::Var(_) => "unknown".into(),
        }
    }

    /// A declared type as a TypeScript type: an alias by its name, a range as `bigint`, a claim as its base.
    pub fn d_ts_type(&mut self, t: &DTy) -> String {
        match t {
            DTy::Unit => "null".into(),
            DTy::Bool => "boolean".into(),
            DTy::Int | DTy::Range { .. } => "bigint".into(),
            DTy::Text => "string".into(),
            DTy::Where(base, _) => self.d_ts_type(base),
            DTy::Named(id) => self.ts_type(&LTy::Named(*id)),
            DTy::Option(x) => format!("{}.Option<{}>", self.rt_type(), self.d_ts_type(x)),
            DTy::List(x) => readonly_array(&self.d_ts_type(x)),
            DTy::Set(x) => format!("{}.OrdSet<{}>", self.rt_type(), self.d_ts_type(x)),
            DTy::Map(k, v) => format!("{}.OrdMap<{}, {}>", self.rt_type(), self.d_ts_type(k), self.d_ts_type(v)),
            DTy::Tuple(xs) => {
                let items: Vec<String> = xs.iter().map(|x| self.d_ts_type(x)).collect();
                format!("readonly [{}]", items.join(", "))
            }
            DTy::Trace(_) => self.ts_type(&LTy::Trace(Vec::new())),
        }
    }

    /// An `LTy` as a runtime type: its order and how it shows.
    pub fn l_desc(&mut self, t: &LTy) -> String {
        let text = self.l_text(t);
        self.note_rt(&text);
        text
    }

    fn l_text(&mut self, t: &LTy) -> String {
        let rt = "rt";
        match t {
            LTy::Unit => format!("{rt}.unit"),
            LTy::Bool => format!("{rt}.bool"),
            LTy::Int => format!("{rt}.int"),
            LTy::Text => format!("{rt}.text"),
            LTy::Named(id) => self.named_desc(*id),
            LTy::Option(x) => format!("{rt}.option({})", self.l_desc(x)),
            LTy::List(x) => format!("{rt}.list({})", self.l_desc(x)),
            LTy::Set(x) => format!("{rt}.set({})", self.l_desc(x)),
            LTy::Map(k, v) => format!("{rt}.map({}, {})", self.l_desc(k), self.l_desc(v)),
            LTy::Tuple(xs) => {
                let items: Vec<String> = xs.iter().map(|x| self.l_desc(x)).collect();
                format!("{rt}.tuple({})", items.join(", "))
            }
            LTy::Range => format!("{rt}.rangeValue"),
            LTy::Func(..) | LTy::Never | LTy::Var(_) => format!("{rt}.func"),
            LTy::Event(_) => format!("{rt}.event"),
            LTy::Reply(_) => format!("{rt}.replies"),
            LTy::Failure => format!("{rt}.failure"),
            LTy::World => self.machine_value("worldType"),
            LTy::Step(_) => format!("new {rt}.StepType({})", self.machine_value("worldType")),
            LTy::Trace(_) => format!("new {rt}.TraceType({})", self.machine_value("worldType")),
        }
    }

    fn named_desc(&mut self, id: TypeId) -> String {
        let rt = "rt";
        if id == self.plan.lir.result_type {
            return format!("{rt}.replies");
        }
        match &self.plan.lir.ty(id).def {
            TypeDef::RuleSet { .. } => format!("{rt}.event"),
            _ => self.type_ref(id),
        }
    }

    /// A declared type as a runtime type: ranges and claims included.
    pub fn d_desc(&mut self, t: &DTy) -> String {
        let text = self.d_text(t);
        self.note_rt(&text);
        text
    }

    fn d_text(&mut self, t: &DTy) -> String {
        let rt = "rt";
        match t {
            DTy::Unit => format!("{rt}.unit"),
            DTy::Bool => format!("{rt}.bool"),
            DTy::Int => format!("{rt}.int"),
            DTy::Text => format!("{rt}.text"),
            DTy::Named(id) => self.named_desc(*id),
            DTy::Option(x) => format!("{rt}.option({})", self.d_desc(x)),
            DTy::List(x) => format!("{rt}.list({})", self.d_desc(x)),
            DTy::Set(x) => format!("{rt}.set({})", self.d_desc(x)),
            DTy::Map(k, v) => format!("{rt}.map({}, {})", self.d_desc(k), self.d_desc(v)),
            DTy::Tuple(xs) => {
                let items: Vec<String> = xs.iter().map(|x| self.d_desc(x)).collect();
                format!("{rt}.tuple({})", items.join(", "))
            }
            DTy::Range { lo, hi, inclusive } => {
                format!("{rt}.inRange({}, {}, {inclusive})", int_literal(*lo), int_literal(*hi))
            }
            DTy::Where(base, claim) => {
                let base = self.d_desc(base);
                let c = self.plan.lir.claim(*claim);
                let self_ty = self.ts_type(&c.self_ty);
                let body = crate::expr::claim_body(self, &c.body);
                format!("{rt}.where({base}, (self: {self_ty}) => {body}, {}, {})", string_literal(&c.text), claim.0)
            }
            DTy::Trace(_) => format!("new {rt}.TraceType({})", self.machine_value("worldType")),
        }
    }
}

/// `readonly T[]`, the element in parentheses where its type is more than a name.
fn readonly_array(inner: &str) -> String {
    if inner.starts_with("readonly ") || inner.contains("=>") {
        format!("readonly ({inner})[]")
    } else {
        format!("readonly {inner}[]")
    }
}

/// Whether every well-typed value conforms: no range and no claim anywhere inside, so no check is printed.
pub fn trivial(plan: &Plan<'_>, t: &DTy) -> bool {
    trivial_in(plan, t, &mut Vec::new())
}

fn trivial_in(plan: &Plan<'_>, t: &DTy, seen: &mut Vec<TypeId>) -> bool {
    match t {
        DTy::Unit | DTy::Bool | DTy::Int | DTy::Text | DTy::Trace(_) => true,
        DTy::Range { .. } | DTy::Where(..) => false,
        DTy::Option(x) | DTy::List(x) | DTy::Set(x) => trivial_in(plan, x, seen),
        DTy::Map(k, v) => trivial_in(plan, k, seen) && trivial_in(plan, v, seen),
        DTy::Tuple(xs) => xs.iter().all(|x| trivial_in(plan, x, seen)),
        DTy::Named(id) => {
            if seen.contains(id) {
                return true;
            }
            seen.push(*id);
            let decl = plan.lir.ty(*id);
            match &decl.def {
                _ if decl.builtin => true,
                TypeDef::Alias { of } => trivial_in(plan, of, seen),
                TypeDef::Record { fields } => fields.iter().all(|f| trivial_in(plan, &f.check, seen)),
                TypeDef::Sum { variants } => {
                    variants.iter().all(|v| v.fields.iter().all(|f| trivial_in(plan, &f.check, seen)))
                }
                TypeDef::RuleSet { .. } => true,
            }
        }
    }
}
