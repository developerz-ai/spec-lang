//! Values drawn from declared types, claims included: the engine's
//! algorithm, step for step, so the same seed gives the same values.

use super::machine::{Halt, Machine};
use super::rng::Rng;
use super::value::Value;
use crate::program::{Lir, TypeDef};
use crate::ty::DTy;
use spec_lang_diag::Diag;
use spec_lang_syntax::Span;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// Tries per value before a claim counts as unmet.
pub const TRIES: u32 = 400;

/// Values already in a world, by declared type: the keys of each state
/// declared `Map[K, V]` and the elements of each declared `Set[K]`.
#[derive(Debug, Default)]
pub struct Pools<'l> {
    pools: Vec<(DTy, BTreeSet<Value<'l>>)>,
}

impl<'l> Pools<'l> {
    pub fn from_worlds<'w>(lir: &Lir, worlds: impl Iterator<Item = &'w Vec<Value<'l>>>) -> Self
    where
        'l: 'w,
    {
        let mut p = Pools::default();
        for w in worlds {
            for (s, v) in lir.states.iter().zip(w) {
                match (&s.check, v) {
                    (DTy::Map(k, _), Value::Map(map)) => p.add(k, map.keys()),
                    (DTy::Set(k), Value::Set(xs)) => p.add(k, xs.iter()),
                    _ => {}
                }
            }
        }
        p
    }

    fn add<'v>(&mut self, ty: &DTy, vals: impl Iterator<Item = &'v Value<'l>>)
    where
        'l: 'v,
    {
        let slot = match self.pools.iter().position(|(t, _)| t == ty) {
            Some(i) => i,
            None => {
                self.pools.push((ty.clone(), BTreeSet::new()));
                self.pools.len() - 1
            }
        };
        self.pools[slot].1.extend(vals.cloned());
    }

    fn get(&self, ty: &DTy) -> Option<Vec<Value<'l>>> {
        self.pools
            .iter()
            .find(|(t, _)| t == ty)
            .map(|(_, v)| v.iter().cloned().collect::<Vec<_>>())
            .filter(|v| !v.is_empty())
    }
}

/// Why no value came out.
#[derive(Debug)]
pub enum GenError<'l> {
    Halt(Halt<'l>),
    Exhausted(String),
}

impl<'l> From<Halt<'l>> for GenError<'l> {
    fn from(h: Halt<'l>) -> Self {
        GenError::Halt(h)
    }
}

impl GenError<'_> {
    pub fn into_diag(self, span: Span) -> Diag {
        match self {
            GenError::Exhausted(ty) => Diag::error("E-GEN", span, format!("no value of `{ty}` found in {TRIES} tries")),
            GenError::Halt(Halt::Fatal(d) | Halt::Check(d) | Halt::Blocked(d)) => *d,
            GenError::Halt(Halt::Fail(f)) => {
                Diag::error("E-GEN", span, format!("a claim failed with {} while generating", f.name))
            }
        }
    }
}

/// A value of `ty`: half the time (when a pool has values of exactly this
/// type) one of them, else a fresh base value; kept when it conforms, else
/// drawn again, at most [`TRIES`] times.
pub fn value<'l>(m: &mut Machine<'l>, rng: &mut Rng, pools: &Pools<'l>, ty: &DTy) -> Result<Value<'l>, GenError<'l>> {
    for _ in 0..TRIES {
        let v = match pools.get(ty) {
            Some(vals) if rng.chance(1, 2) => rng.pick(&vals).cloned().unwrap_or(Value::Unit),
            _ => base(m, rng, pools, ty)?,
        };
        if m.conform(&v, ty)?.is_none() {
            return Ok(v);
        }
    }
    Err(GenError::Exhausted(format!("{ty:?}")))
}

fn base<'l>(m: &mut Machine<'l>, rng: &mut Rng, pools: &Pools<'l>, ty: &DTy) -> Result<Value<'l>, GenError<'l>> {
    Ok(match ty {
        DTy::Unit => Value::Unit,
        DTy::Bool => Value::Bool(rng.chance(1, 2)),
        DTy::Int => Value::Int(int(rng)),
        DTy::Text => Value::text(&text(rng)),
        DTy::Range { lo, hi, inclusive } => {
            let hi = if *inclusive { *hi } else { hi - 1 };
            Value::Int(if rng.chance(1, 5) { if rng.chance(1, 2) { *lo } else { hi } } else { rng.between(*lo, hi) })
        }
        DTy::Option(inner) => {
            if rng.chance(1, 3) {
                m.none()
            } else {
                let v = value(m, rng, pools, inner)?;
                m.some(v)
            }
        }
        DTy::List(t) => {
            let n = rng.below(4);
            let mut xs = Vec::new();
            for _ in 0..n {
                xs.push(value(m, rng, pools, t)?);
            }
            Value::list(xs)
        }
        DTy::Set(t) => {
            let n = rng.below(4);
            let mut xs = BTreeSet::new();
            for _ in 0..n {
                xs.insert(value(m, rng, pools, t)?);
            }
            Value::Set(Rc::new(xs))
        }
        DTy::Map(k, v) => {
            let n = rng.below(4);
            let mut map = BTreeMap::new();
            for _ in 0..n {
                let key = value(m, rng, pools, k)?;
                let val = value(m, rng, pools, v)?;
                map.insert(key, val);
            }
            Value::Map(Rc::new(map))
        }
        DTy::Tuple(ts) => {
            let mut xs = Vec::new();
            for t in ts {
                xs.push(value(m, rng, pools, t)?);
            }
            Value::Tuple(Rc::new(xs))
        }
        DTy::Where(b, _) => value(m, rng, pools, b)?,
        DTy::Named(t) => {
            let lir = m.lir;
            match &lir.ty(*t).def {
                TypeDef::Alias { of } => value(m, rng, pools, of)?,
                TypeDef::Record { fields } => {
                    let mut xs = Vec::new();
                    for f in fields {
                        xs.push(value(m, rng, pools, &f.check)?);
                    }
                    Value::Record(*t, Rc::new(xs))
                }
                TypeDef::Sum { variants } if !lir.ty(*t).builtin => {
                    let tag = rng.below(variants.len() as u64);
                    let mut xs = Vec::new();
                    for f in variants.get(tag as usize).map(|v| v.fields.as_slice()).unwrap_or(&[]) {
                        xs.push(value(m, rng, pools, &f.check)?);
                    }
                    Value::Variant(*t, u32::try_from(tag).unwrap_or(0), Rc::new(xs))
                }
                TypeDef::Sum { .. } => return Err(GenError::Exhausted("a built-in sum".into())),
                TypeDef::RuleSet { .. } => return Err(GenError::Exhausted("a rule set".into())),
            }
        }
        DTy::Trace(_) => return Err(GenError::Exhausted("Trace".into())),
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

/// Text from one of four alphabets: shell-safe words, printable ASCII, a mix
/// with Unicode space, controls and astral characters, or empty.
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
