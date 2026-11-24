//! Values generated from types. A claim is met by generating its base and
//! keeping the values the claim accepts; a type whose claim rejects every
//! try is an `E-GEN` error, never a silent skip.

use crate::rng::Rng;
use spec_lang_diag::Diag;
use spec_lang_engine::{Halt, Interp, Value, World};
use spec_lang_ir::*;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// Tries per value before giving up on a claim.
const TRIES: u32 = 400;

/// Values already in the world, by type: map keys and set elements. A
/// generated argument of the same type picks one of them half of the time,
/// so `done(no)` mostly names a task that exists.
#[derive(Debug, Default)]
pub(crate) struct Pools {
    pools: Vec<(Ty, BTreeSet<Value>)>,
}

impl Pools {
    pub(crate) fn from_worlds<'w>(prog: &Program, worlds: impl Iterator<Item = &'w World>) -> Self {
        let mut p = Pools::default();
        for w in worlds {
            for (s, v) in prog.states.iter().zip(&w.states) {
                match (&s.ty, v) {
                    (Ty::Map(k, _), Value::Map(m)) => p.add(k, m.keys()),
                    (Ty::Set(k), Value::Set(xs)) => p.add(k, xs.iter()),
                    _ => {}
                }
            }
        }
        p
    }

    fn add<'v>(&mut self, ty: &Ty, vals: impl Iterator<Item = &'v Value>) {
        let slot = match self.pools.iter().position(|(t, _)| t == ty) {
            Some(i) => i,
            None => {
                self.pools.push((ty.clone(), BTreeSet::new()));
                self.pools.len() - 1
            }
        };
        self.pools[slot].1.extend(vals.cloned());
    }

    fn get(&self, ty: &Ty) -> Option<Vec<Value>> {
        self.pools
            .iter()
            .find(|(t, _)| t == ty)
            .map(|(_, v)| v.iter().cloned().collect())
            .filter(|v: &Vec<Value>| !v.is_empty())
    }
}

/// Why no value came out.
#[derive(Debug)]
pub(crate) enum GenError {
    Halt(Halt),
    Exhausted(String),
}

impl GenError {
    pub(crate) fn into_diag(self, span: spec_lang_syntax::Span) -> Diag {
        match self {
            GenError::Exhausted(ty) => Diag::error("E-GEN", span, format!("no value of `{ty}` found in {TRIES} tries"))
                .with_note("a tight claim needs a generator; until then this check cannot run"),
            GenError::Halt(Halt::Fatal(d) | Halt::Check(d)) => *d,
            GenError::Halt(Halt::Fail(f)) => {
                Diag::error("E-GEN", span, format!("a claim failed with {} while generating", f.name))
            }
            GenError::Halt(Halt::Blocked(d)) => d.with_note("while generating a value"),
        }
    }
}

impl From<Halt> for GenError {
    fn from(h: Halt) -> Self {
        GenError::Halt(h)
    }
}

/// A value of `ty`, claims included.
pub(crate) fn value(it: &mut Interp<'_>, rng: &mut Rng, pools: &Pools, ty: &Ty) -> Result<Value, GenError> {
    for _ in 0..TRIES {
        let v = match pools.get(ty) {
            Some(vals) if rng.chance(1, 2) => rng.pick(&vals).cloned().unwrap_or(Value::Unit),
            _ => base(it, rng, pools, ty)?,
        };
        if it.conform(&v, ty)?.is_none() {
            return Ok(v);
        }
    }
    Err(GenError::Exhausted(spec_lang_engine::display::show_ty(it.prog, ty)))
}

fn base(it: &mut Interp<'_>, rng: &mut Rng, pools: &Pools, ty: &Ty) -> Result<Value, GenError> {
    Ok(match ty {
        Ty::Unit => Value::Unit,
        Ty::Bool => Value::Bool(rng.chance(1, 2)),
        Ty::Int => Value::Int(int(rng)),
        Ty::Text => Value::text(&text(rng)),
        Ty::Range { lo, hi, inclusive } => {
            let hi = if *inclusive { *hi } else { hi - 1 };
            Value::Int(if rng.chance(1, 5) { if rng.chance(1, 2) { *lo } else { hi } } else { rng.between(*lo, hi) })
        }
        Ty::Option(inner) => {
            if rng.chance(1, 3) {
                it.empty_of(ty).unwrap_or(Value::Unit)
            } else {
                Value::Variant(it.prog.option_type, SOME_TAG, Rc::new(vec![value(it, rng, pools, inner)?]))
            }
        }
        Ty::List(t) => {
            let n = rng.below(4);
            let mut xs = Vec::new();
            for _ in 0..n {
                xs.push(value(it, rng, pools, t)?);
            }
            Value::list(xs)
        }
        Ty::Set(t) => {
            let n = rng.below(4);
            let mut xs = BTreeSet::new();
            for _ in 0..n {
                xs.insert(value(it, rng, pools, t)?);
            }
            Value::Set(Rc::new(xs))
        }
        Ty::Map(k, v) => {
            let n = rng.below(4);
            let mut m = BTreeMap::new();
            for _ in 0..n {
                m.insert(value(it, rng, pools, k)?, value(it, rng, pools, v)?);
            }
            Value::Map(Rc::new(m))
        }
        Ty::Tuple(ts) => {
            let mut xs = Vec::new();
            for t in ts {
                xs.push(value(it, rng, pools, t)?);
            }
            Value::Tuple(Rc::new(xs))
        }
        Ty::Where(b, _) => value(it, rng, pools, b)?,
        Ty::Named(t) => match &it.prog.ty(*t).def {
            TypeDef::Alias(a) => {
                let a = a.clone();
                value(it, rng, pools, &a)?
            }
            TypeDef::Record(fields) => {
                let tys: Vec<Ty> = fields.iter().map(|f| f.ty.clone()).collect();
                let mut xs = Vec::new();
                for ft in &tys {
                    xs.push(value(it, rng, pools, ft)?);
                }
                Value::Record(*t, Rc::new(xs))
            }
            TypeDef::Sum(vs) => {
                let tag = rng.below(vs.len() as u64);
                let tys: Vec<Ty> =
                    vs.get(tag as usize).map(|v| v.fields.iter().map(|f| f.ty.clone()).collect()).unwrap_or_default();
                let mut xs = Vec::new();
                for ft in &tys {
                    xs.push(value(it, rng, pools, ft)?);
                }
                Value::Variant(*t, u32::try_from(tag).unwrap_or(0), Rc::new(xs))
            }
            TypeDef::Rules(_) => return Err(GenError::Exhausted("a rule set".into())),
        },
        Ty::Trace(_) => return Err(GenError::Exhausted("Trace".into())),
    })
}

/// Small numbers most of the time, then boundaries a machine word breaks at.
fn int(rng: &mut Rng) -> i128 {
    const EDGES: &[i128] = &[
        0,
        1,
        -1,
        2_147_483_647,
        2_147_483_648,
        9_007_199_254_740_992,
        9_007_199_254_740_993,
        9_223_372_036_854_775_807,
        9_223_372_036_854_775_808,
        -9_223_372_036_854_775_808,
        18_446_744_073_709_551_616,
    ];
    match rng.below(100) {
        0..35 => rng.between(0, 10),
        35..55 => rng.between(11, 100),
        55..70 => rng.between(-10, -1),
        70..80 => rng.pick(EDGES).copied().unwrap_or(0),
        80..90 => rng.between(101, 1 << 62),
        _ => -rng.between(11, 1 << 62),
    }
}

/// Text from one of four alphabets: shell-safe words, printable ASCII,
/// a mix with Unicode space, controls and astral characters, or empty.
fn text(rng: &mut Rng) -> String {
    const MIX: &[char] = &[
        'a', 'Z', ' ', '\t', '\n', 'é', 'ß', '中', '😀', '\u{3000}', '\u{85}', '\u{A0}', '\u{200B}', '\u{FEFF}',
        '\u{7F}', '\u{1}', '"', '\\', '{', '}', '#', '-',
    ];
    let pick = |rng: &mut Rng, n: u64, f: &dyn Fn(&mut Rng) -> char| (0..n).map(|_| f(rng)).collect::<String>();
    match rng.below(4) {
        0 => {
            let n = 1 + rng.below(8);
            pick(rng, n, &|r| *r.pick(b"abcdefghijklmnopqrstuvwxyz0123456789-").unwrap_or(&b'a') as char)
        }
        1 => {
            let n = rng.below(13);
            pick(rng, n, &|r| char::from(u8::try_from(32 + r.below(95)).unwrap_or(b'a')))
        }
        2 => {
            let n = rng.below(9);
            pick(rng, n, &|r| *r.pick(MIX).unwrap_or(&'a'))
        }
        _ => String::new(),
    }
}
