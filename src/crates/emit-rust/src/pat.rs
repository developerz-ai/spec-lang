//! Patterns: the LIR's steps become an `if let` chain over references into the scrutinee, then bindings.

use crate::code::{Code, Fx};
use crate::names::{ident, local};
use spec_lang_lir::node::{Arm, Expr, Pat, PatStep, PathStep};
use spec_lang_lir::{LTy, TypeDef};
use std::collections::BTreeMap;

/// A value inside the scrutinee: an expression of type `&T`, and one on which `.field` works.
#[derive(Clone, Debug)]
struct Node {
    refx: String,
    base: String,
    /// `None`: a reply's payload, still the `Out` enum.
    ty: Option<LTy>,
}

impl Node {
    /// The value itself, as a place.
    fn deref(&self) -> String {
        match self.refx.strip_prefix('&') {
            Some(p) => p.to_string(),
            None => format!("*{}", self.refx),
        }
    }
}

/// One element of an `if let` chain.
enum Link {
    Let { pat: String, of: String },
    Cond(String),
}

struct Chain {
    links: Vec<Link>,
    binds: Vec<String>,
    nodes: BTreeMap<String, Node>,
    m: bool,
    fail: bool,
}

impl Chain {
    fn text(&self) -> String {
        let parts: Vec<String> = self
            .links
            .iter()
            .map(|l| match l {
                Link::Let { pat, of } => format!("let {pat} = {of}"),
                Link::Cond(c) => c.clone(),
            })
            .collect();
        if parts.is_empty() { "true".into() } else { parts.join(" && ") }
    }
}

impl Fx<'_, '_> {
    fn node(&mut self, ch: &mut Chain, root: &Node, path: &[PathStep], want: Option<&LTy>) -> Node {
        let key = format!("{path:?} {want:?}");
        if let Some(n) = ch.nodes.get(&key) {
            return n.clone();
        }
        let lir = self.cx.lir;
        let n = match path.split_last() {
            None => root.clone(),
            Some((step, prefix)) => {
                let parent = self.node(ch, root, prefix, None);
                let wanted = want_for_parent(lir, step);
                let parent = self.unwrap_out(ch, parent, prefix, wanted.as_ref());
                let p = self.tmp();
                match step {
                    PathStep::TupleItem(i) => {
                        let ty = match &parent.ty {
                            Some(LTy::Tuple(xs)) => xs.get(*i as usize).cloned(),
                            _ => None,
                        };
                        Node { refx: format!("&{}.{i}", parent.base), base: format!("{}.{i}", parent.base), ty }
                    }
                    PathStep::EventArg { rule, index } => {
                        let r = lir.rule(*rule);
                        let pname = r.params.get(*index as usize).map_or("x".to_string(), |p| ident(&p.name));
                        ch.links.push(Link::Let {
                            pat: format!("crate::machine::Event::{} {{ {pname}: {p}, .. }}", self.cx.names.event(*rule)),
                            of: parent.refx.clone(),
                        });
                        let ty = r.params.get(*index as usize).map(|x| x.ty.clone());
                        Node { refx: p.clone(), base: p, ty }
                    }
                    PathStep::VariantField { ty, tag, index } => {
                        if *ty == lir.option_type {
                            ch.links.push(Link::Let { pat: format!("Opt::Some({p})"), of: parent.refx.clone() });
                            let inner = match &parent.ty {
                                Some(LTy::Option(x)) => Some((**x).clone()),
                                _ => None,
                            };
                            Node { refx: p.clone(), base: p, ty: inner }
                        } else if *ty == lir.result_type {
                            let (v, ty) = if *tag == 0 { ("Ok", None) } else { ("Err", Some(LTy::Failure)) };
                            ch.links.push(Link::Let { pat: format!("crate::machine::Reply::{v}({p})"), of: parent.refx.clone() });
                            Node { refx: p.clone(), base: p, ty }
                        } else {
                            let (fname, fty) = match &lir.ty(*ty).def {
                                TypeDef::Sum { variants } => variants
                                    .get(*tag as usize)
                                    .and_then(|v| v.fields.get(*index as usize))
                                    .map_or(("x".to_string(), None), |f| (ident(&f.name), Some(f.ty.clone()))),
                                _ => ("x".to_string(), None),
                            };
                            let path = self.cx.type_path(*ty, self.module);
                            ch.links.push(Link::Let {
                                pat: format!("{path}::{} {{ {fname}: {p}, .. }}", self.variant_name(*ty, *tag)),
                                of: parent.refx.clone(),
                            });
                            if self.cx.is_boxed(*ty, *tag, *index) {
                                Node { refx: format!("&**{p}"), base: p, ty: fty }
                            } else {
                                Node { refx: p.clone(), base: p, ty: fty }
                            }
                        }
                    }
                }
            }
        };
        let n = self.unwrap_out(ch, n, path, want);
        ch.nodes.insert(key, n.clone());
        n
    }

    /// A reply's payload as the type the pattern wants.
    fn unwrap_out(&mut self, ch: &mut Chain, n: Node, _path: &[PathStep], want: Option<&LTy>) -> Node {
        match (&n.ty, want) {
            (None, Some(t)) => match self.cx.names.out(t).map(str::to_string) {
                Some(variant) => {
                    let p = self.tmp();
                    ch.links.push(Link::Let { pat: format!("crate::machine::Out::{variant}({p})"), of: n.refx.clone() });
                    Node { refx: p.clone(), base: p, ty: Some(t.clone()) }
                }
                None => n,
            },
            _ => n,
        }
    }

    fn chain(&mut self, root: &Node, pat: &Pat, bind: bool) -> Chain {
        let mut ch = Chain { links: Vec::new(), binds: Vec::new(), nodes: BTreeMap::new(), m: false, fail: false };
        let lir = self.cx.lir;
        for step in &pat.steps {
            match step {
                PatStep::Tag { path, ty, tag } => {
                    let want = if *ty == lir.option_type || *ty == lir.result_type { None } else { Some(LTy::Named(*ty)) };
                    let n = self.node(&mut ch, root, path, want.as_ref());
                    let pat = if *ty == lir.option_type {
                        if *tag == spec_lang_ir::SOME_TAG { "Opt::Some(_)".to_string() } else { "Opt::None".to_string() }
                    } else if *ty == lir.result_type {
                        if *tag == spec_lang_ir::OK_TAG { "crate::machine::Reply::Ok(_)".into() } else { "crate::machine::Reply::Err(_)".into() }
                    } else {
                        let has_fields = match &lir.ty(*ty).def {
                            TypeDef::Sum { variants } => variants.get(*tag as usize).is_some_and(|v| !v.fields.is_empty()),
                            _ => false,
                        };
                        let path = self.cx.type_path(*ty, self.module);
                        let v = self.variant_name(*ty, *tag);
                        if has_fields { format!("{path}::{v} {{ .. }}") } else { format!("{path}::{v}") }
                    };
                    ch.links.push(Link::Let { pat, of: n.refx });
                }
                PatStep::Event { path, rule } => {
                    let n = self.node(&mut ch, root, path, None);
                    ch.links.push(Link::Let { pat: format!("crate::machine::Event::{} {{ .. }}", self.cx.names.event(*rule)), of: n.refx });
                }
                PatStep::Equals { path, value } => {
                    let n = self.node(&mut ch, root, path, Some(&value.ty));
                    let v = self.value(value);
                    ch.m |= v.m;
                    ch.fail |= v.fail;
                    ch.links.push(Link::Cond(format!("{} == {}", n.deref(), v.sealed().operand())));
                }
                PatStep::Never => ch.links.push(Link::Cond("false".into())),
                PatStep::Bind { name, path, ty } => {
                    if !bind {
                        continue;
                    }
                    let n = self.node(&mut ch, root, path, Some(ty));
                    let d = n.deref();
                    let owned = if self.cx.copy(ty) {
                        d
                    } else if d.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                        format!("{d}.clone()")
                    } else {
                        format!("({d}).clone()")
                    };
                    ch.binds.push(format!("let {} = {owned};", local(name)));
                }
            }
        }
        ch
    }

    /// The scrutinee as a root node: a local is read in place, anything else is evaluated once into a temporary.
    fn root(&mut self, value: &Expr, pre: &mut Vec<String>) -> (Node, bool, bool) {
        let c = self.expr(value);
        pre.extend(c.pre.iter().cloned());
        if c.place && !c.m {
            return (Node { refx: format!("&{}", c.text), base: c.text.clone(), ty: Some(value.ty.clone()) }, false, c.fail);
        }
        let t = self.tmp();
        let owned = if c.place && !c.copy { format!("{}.clone()", c.operand()) } else { c.text.clone() };
        pre.push(format!("let {t} = {owned};"));
        (Node { refx: format!("&{t}"), base: t, ty: Some(value.ty.clone()) }, c.m, c.fail)
    }

    pub fn is(&mut self, value: &Expr, pat: &Pat) -> Code {
        let mut pre = Vec::new();
        let (root, m, fail) = self.root(value, &mut pre);
        let ch = self.chain(&root, pat, false);
        let text = match ch.links.as_slice() {
            [] => "true".to_string(),
            [Link::Let { pat, of }, rest @ ..] if rest.iter().all(|l| matches!(l, Link::Cond(_))) => {
                let conds: Vec<String> = rest.iter().filter_map(|l| if let Link::Cond(c) = l { Some(c.clone()) } else { None }).collect();
                if conds.is_empty() { format!("matches!({of}, {pat})") } else { format!("matches!({of}, {pat} if {})", conds.join(" && ")) }
            }
            links if links.iter().all(|l| matches!(l, Link::Cond(_))) => ch.text(),
            _ => format!("if {} {{ true }} else {{ false }}", ch.text()),
        };
        Code { pre, text, atomic: false, m: m || ch.m, fail: fail || ch.fail, ..Code::default() }
    }

    pub fn match_(&mut self, scrutinee: &Expr, arms: &[Arm], ty: &LTy) -> Code {
        let mut pre = Vec::new();
        let (root, mut m, mut fail) = self.root(scrutinee, &mut pre);
        let mut text = String::new();
        let mut closed = false;
        for (i, arm) in arms.iter().enumerate() {
            let ch = self.chain(&root, &arm.pat, true);
            let v = self.value(&arm.value);
            m |= ch.m || v.m;
            fail |= ch.fail || v.fail;
            let mut body = ch.binds.clone();
            body.extend(v.pre.iter().cloned());
            body.push(v.text.clone());
            let body = body.join(" ");
            if ch.links.is_empty() {
                text = if i == 0 { format!("{{ {body} }}") } else { format!("{text} else {{ {body} }}") };
                closed = true;
                break;
            }
            let head = if i == 0 { "if" } else { " else if" };
            text.push_str(&format!("{head} {} {{ {body} }}", ch.text()));
        }
        if !closed {
            let _ = ty;
            text.push_str(" else { return Err(rt::stop(\"E-RUNTIME\", \"no arm matches\")) }");
            fail = true;
        }
        Code { pre, text, atomic: true, m, fail, ..Code::default() }
    }
}

/// What a path step needs its parent to be when the parent is a reply's payload.
fn want_for_parent(lir: &spec_lang_lir::Lir, step: &PathStep) -> Option<LTy> {
    match step {
        PathStep::VariantField { ty, .. } if *ty != lir.option_type && *ty != lir.result_type => Some(LTy::Named(*ty)),
        _ => None,
    }
}
