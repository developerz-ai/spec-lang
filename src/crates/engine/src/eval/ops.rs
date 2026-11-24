//! Operators, comparisons and indexing.

use super::{Env, runtime};
use crate::display::show;
use crate::error::{Halt, Halted};
use crate::interp::Interp;
use crate::value::Value;
use spec_lang_diag::Diag;
use spec_lang_ir::*;
use spec_lang_syntax::Span;
use std::rc::Rc;

fn overflow(span: Span) -> Halt {
    runtime(span, "an Int beyond the engine's 128-bit range (the language's Int is unbounded; this engine is not)")
}

impl Interp<'_> {
    pub(crate) fn unary(&mut self, op: UnOp, v: Value, span: Span) -> Halted<Value> {
        match (op, v) {
            (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
            (UnOp::Neg, Value::Int(n)) => n.checked_neg().map(Value::Int).ok_or_else(|| overflow(span)),
            (UnOp::Pos, Value::Int(n)) => Ok(Value::Int(n)),
            (op, v) => {
                Err(Diag::error("E-TYPE-MISMATCH", span, format!("`{op:?}` does not apply to {}", show(self.prog, &v)))
                    .into())
            }
        }
    }

    fn bool_of(&mut self, env: &mut Env, e: &Expr) -> Halted<bool> {
        match self.eval(env, e)? {
            Value::Bool(b) => Ok(b),
            v => Err(self.not_bool(&v, e.span)),
        }
    }

    pub(crate) fn binary(&mut self, env: &mut Env, op: BinOp, l: &Expr, r: &Expr, span: Span) -> Halted<Value> {
        match op {
            BinOp::And => return Ok(Value::Bool(self.bool_of(env, l)? && self.bool_of(env, r)?)),
            BinOp::Or => return Ok(Value::Bool(self.bool_of(env, l)? || self.bool_of(env, r)?)),
            BinOp::Implies => return Ok(Value::Bool(!self.bool_of(env, l)? || self.bool_of(env, r)?)),
            _ => {}
        }
        let a = self.eval(env, l)?;
        let b = self.eval(env, r)?;
        self.arith(op, a, b, span)
    }

    fn arith(&mut self, op: BinOp, a: Value, b: Value, span: Span) -> Halted<Value> {
        use Value::Int;
        let int = |r: Option<i128>| r.map(Int).ok_or_else(|| overflow(span));
        match (op, &a, &b) {
            (BinOp::Add, Int(x), Int(y)) => int(x.checked_add(*y)),
            (BinOp::Sub, Int(x), Int(y)) => int(x.checked_sub(*y)),
            (BinOp::Mul, Int(x), Int(y)) => int(x.checked_mul(*y)),
            (BinOp::Div, Int(_), Int(_)) => {
                Err(Diag::error("E-OPEN", span, "`/` on Int is an open choice: say how to round")
                    .with_alternative("rounds down", Vec::new())
                    .with_alternative("truncates toward zero", Vec::new())
                    .with_note("write `a.div(b, floor)` or `a.div(b, trunc)`")
                    .into())
            }
            (BinOp::Mod, Int(_), Int(0)) => Err(Halt::fail("DivByZero")),
            (BinOp::Mod, Int(x), Int(y)) => {
                int(x.checked_rem_euclid(*y).map(|m| if *y < 0 && m != 0 { m + y } else { m }))
            }
            (BinOp::Pow, Int(x), Int(y)) if *y >= 0 => int(u32::try_from(*y).ok().and_then(|e| x.checked_pow(e))),
            (BinOp::Band | BinOp::Bor | BinOp::Bxor | BinOp::Shl | BinOp::Shr, Int(x), Int(y))
                if *x >= 0 && *y >= 0 =>
            {
                match op {
                    BinOp::Band => Ok(Int(x & y)),
                    BinOp::Bor => Ok(Int(x | y)),
                    BinOp::Bxor => Ok(Int(x ^ y)),
                    BinOp::Shl => int(u32::try_from(*y).ok().and_then(|s| x.checked_shl(s)).filter(|v| v >> y == *x)),
                    _ => Ok(Int(u32::try_from(*y).ok().and_then(|s| x.checked_shr(s)).unwrap_or(0))),
                }
            }
            (BinOp::Add, Value::List(x), Value::List(y)) => {
                let mut v = (**x).clone();
                v.extend(y.iter().cloned());
                Ok(Value::list(v))
            }
            (BinOp::Add, Value::Set(x), Value::Set(y)) => Ok(Value::Set(Rc::new(x.union(y).cloned().collect()))),
            (BinOp::Sub, Value::Set(x), Value::Set(y)) => Ok(Value::Set(Rc::new(x.difference(y).cloned().collect()))),
            (BinOp::Add, Value::Text(_), Value::Text(_)) => {
                Err(Diag::error("E-TYPE-MISMATCH", span, "`+` does not join Text").with_note("write \"{a}{b}\"").into())
            }
            _ => Err(Diag::error(
                "E-TYPE-MISMATCH",
                span,
                format!("`{op:?}` does not apply to {} and {}", show(self.prog, &a), show(self.prog, &b)),
            )
            .into()),
        }
    }

    /// `a op b op c`: every link must hold; stops at the first that does not.
    pub(crate) fn compare(&mut self, env: &mut Env, first: &Expr, rest: &[(CmpOp, Expr)]) -> Halted<Value> {
        let mut left = self.eval(env, first)?;
        for (op, e) in rest {
            let right = self.eval(env, e)?;
            if !self.compare_one(*op, &left, &right, e.span)? {
                return Ok(Value::Bool(false));
            }
            left = right;
        }
        Ok(Value::Bool(true))
    }

    fn compare_one(&mut self, op: CmpOp, a: &Value, b: &Value, span: Span) -> Halted<bool> {
        match op {
            CmpOp::Eq | CmpOp::Ne => {
                self.need_comparable(a, span)?;
                self.need_comparable(b, span)?;
                Ok((a == b) == (op == CmpOp::Eq))
            }
            CmpOp::In => self.contains(b, a, span),
            _ => {
                let ord = match (a, b) {
                    (Value::Int(x), Value::Int(y)) => x.cmp(y),
                    (Value::Text(x), Value::Text(y)) => x.cmp(y),
                    _ => {
                        return Err(Diag::error(
                            "E-TYPE-MISMATCH",
                            span,
                            format!(
                                "`<` compares numbers or Text, not {} and {}",
                                show(self.prog, a),
                                show(self.prog, b)
                            ),
                        )
                        .into());
                    }
                };
                Ok(match op {
                    CmpOp::Lt => ord.is_lt(),
                    CmpOp::Le => ord.is_le(),
                    CmpOp::Gt => ord.is_gt(),
                    _ => ord.is_ge(),
                })
            }
        }
    }

    /// `x in c`.
    pub(crate) fn contains(&mut self, c: &Value, x: &Value, span: Span) -> Halted<bool> {
        match c {
            Value::List(xs) => Ok(xs.contains(x)),
            Value::Set(xs) => Ok(xs.contains(x)),
            Value::Map(m) => Ok(m.contains_key(x)),
            Value::Range(lo, hi, incl) => Ok(match x {
                Value::Int(n) => *n >= *lo && if *incl { *n <= *hi } else { *n < *hi },
                _ => false,
            }),
            Value::Text(t) => match x {
                Value::Text(s) => Ok(t.contains(&**s)),
                _ => Err(runtime(span, "`in` on Text looks for Text")),
            },
            _ => Err(Diag::error(
                "E-TYPE-MISMATCH",
                span,
                format!("`in` needs a collection, found {}", show(self.prog, c)),
            )
            .into()),
        }
    }

    /// `c[k]`: fails `Missing` when absent.
    pub(crate) fn index(&mut self, c: &Value, k: &Value, span: Span) -> Halted<Value> {
        match (c, k) {
            (Value::Map(m), _) => m.get(k).cloned().ok_or_else(Halt::missing),
            (Value::List(xs), Value::Int(i)) => {
                usize::try_from(*i).ok().and_then(|i| xs.get(i)).cloned().ok_or_else(Halt::missing)
            }
            (Value::Text(_), _) => {
                Err(Diag::error("E-TYPE-MISMATCH", span, "Text has no index").with_note("use `take` and `drop`").into())
            }
            _ => Err(Diag::error("E-TYPE-MISMATCH", span, format!("{} has no index", show(self.prog, c))).into()),
        }
    }
}
