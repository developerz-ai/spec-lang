//! Declared types as Ruby values (`Spec::Type`), and whether a check against one can ever fail.

use crate::gen_::{Fx, R};
use spec_lang_ir::TypeId;
use spec_lang_lir::{DTy, LTy, TypeDef};

impl Fx<'_> {
    /// Whether a value of the right `LTy` can be outside `t`: a range or a claim inside. A record or a sum
    /// checks its own fields when it is made, so naming one adds nothing.
    pub(crate) fn strict(&self, t: &DTy) -> bool {
        self.strict_in(t, &mut Vec::new())
    }

    fn strict_in(&self, t: &DTy, seen: &mut Vec<TypeId>) -> bool {
        match t {
            DTy::Unit | DTy::Bool | DTy::Int | DTy::Text | DTy::Trace(_) => false,
            DTy::Range { .. } | DTy::Where(..) => true,
            DTy::Option(t) | DTy::List(t) | DTy::Set(t) => self.strict_in(t, seen),
            DTy::Map(k, v) => self.strict_in(k, seen) || self.strict_in(v, seen),
            DTy::Tuple(ts) => ts.iter().any(|t| self.strict_in(t, seen)),
            DTy::Named(id) => match &self.g.lir.ty(*id).def {
                TypeDef::Alias { of } if !seen.contains(id) => {
                    seen.push(*id);
                    self.strict_in(of, seen)
                }
                _ => false,
            },
        }
    }

    /// The `Spec::Type` of a declared type.
    pub(crate) fn dty(&mut self, t: &DTy) -> R<String> {
        Ok(match t {
            DTy::Unit => "Spec::Type::UNIT".into(),
            DTy::Bool => "Spec::Type::BOOL".into(),
            DTy::Int => "Spec::Type::INT".into(),
            DTy::Text => "Spec::Type::TEXT".into(),
            DTy::Trace(_) => "Spec::Type::TRACE".into(),
            DTy::Named(id) => self.type_ref(*id),
            DTy::Option(t) => format!("Spec::Type.option({})", self.dty(t)?),
            DTy::List(t) => format!("Spec::Type.list({})", self.dty(t)?),
            DTy::Set(t) => format!("Spec::Type.set({})", self.dty(t)?),
            DTy::Map(k, v) => format!("Spec::Type.map({}, {})", self.dty(k)?, self.dty(v)?),
            DTy::Tuple(ts) => {
                let items = ts.iter().map(|t| self.dty(t)).collect::<R<Vec<_>>>()?;
                format!("Spec::Type.tuple({})", items.join(", "))
            }
            DTy::Range { lo, hi, inclusive: true } => format!("Spec::Type.range({lo}, {hi})"),
            DTy::Range { lo, hi, inclusive: false } => format!("Spec::Type.range({lo}, {hi}, exclusive: true)"),
            DTy::Where(base, claim) => {
                let c = self.g.lir.claim(*claim);
                let base = self.dty(base)?;
                self.push();
                let this = self.bind_param("self");
                let body = self.expr(&c.body);
                self.pop();
                let head = format!("Spec::Type.where({}, {}, -> {{ {base} }})", claim.0, crate::code::string(&c.text));
                crate::code::block(&head, &[this], &[body?.text]).text
            }
        })
    }

    /// `T.check(value, "what")` when the check can fail, else the value.
    pub(crate) fn checked(&mut self, t: &DTy, value: String, what: &str) -> R<String> {
        if !self.strict(t) {
            return Ok(value);
        }
        Ok(crate::code::call(&format!("{}.check", self.dty_recv(t)?), &[value, crate::code::string(what)]).text)
    }

    fn dty_recv(&mut self, t: &DTy) -> R<String> {
        let d = self.dty(t)?;
        Ok(if d.contains('\n') || d.contains(" do") || d.contains(" {") { format!("({d})") } else { d })
    }

    /// The value a state without an initial value starts with.
    pub(crate) fn empty_of(t: &LTy) -> Option<&'static str> {
        match t {
            LTy::Map(..) => Some("Spec::Maps::EMPTY"),
            LTy::Set(_) => Some("Spec::Sets::EMPTY"),
            LTy::List(_) => Some("[]"),
            LTy::Option(_) => Some("Spec::NONE"),
            _ => None,
        }
    }
}
