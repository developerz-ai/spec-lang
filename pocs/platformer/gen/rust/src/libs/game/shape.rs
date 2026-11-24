//! `app A(start: .., tick: .., input: .., frame: .., dump: .., report: ..)`, `@bin`, `@window` and the tick rate, read
//! from the manifest, and the commands derived from them.

use crate::rt::Int;
use crate::rt::app::{App, Decl, Func, NoteValue, Param, Rule};
use crate::rt::data::{Data, Shape as TyShape};

#[derive(Debug)]
pub struct Shape {
    pub app: &'static App,
    pub bin: &'static str,
    pub start: &'static Rule,
    pub tick: &'static Rule,
    pub input: &'static Func,
    pub frame: &'static Rule,
    pub dump: &'static Func,
    pub report: &'static Func,
    /// The `Bool` fields of the tick rule's one parameter: the controls of one tick.
    pub controls: Vec<&'static str>,
    pub width: i32,
    pub height: i32,
    pub title: &'static str,
    /// Ticks per second.
    pub rate: i32,
}

fn rule(app: &'static App, field: &str) -> Result<&'static Rule, String> {
    match app.param(field) {
        [Decl::Rule(i)] => app.rules.get(*i).ok_or_else(|| format!("the app's `{field}` names no rule")),
        _ => Err(format!("the app's `{field}` names no rule")),
    }
}

fn func(app: &'static App, field: &str) -> Result<&'static Func, String> {
    match app.param(field) {
        [Decl::Fn(i)] => app.fns.get(*i).ok_or_else(|| format!("the app's `{field}` names no fn")),
        _ => Err(format!("the app's `{field}` names no fn")),
    }
}

fn number(v: Option<&NoteValue>, what: &str) -> Result<i32, String> {
    let n: Option<Int> = match v {
        Some(NoteValue::Int(n) | NoteValue::Quantity(n, _)) => Some(*n),
        _ => None,
    };
    n.and_then(|n| i32::try_from(n).ok()).ok_or_else(|| format!("the app gives no {what}"))
}

impl Shape {
    pub fn of(app: &'static App) -> Result<Self, String> {
        let tick = rule(app, "tick")?;
        let controls = match tick.params {
            [Param { ty, .. }] => match &ty.shape {
                TyShape::Record(fields) => fields.iter().map(|(n, _)| *n).collect(),
                _ => return Err("the tick rule's parameter is not a record of controls".into()),
            },
            _ => return Err("the tick rule takes one parameter, the controls".into()),
        };
        let window = app.note("window").ok_or("the app has no @window")?;
        let rate = tick.triggers.iter().find(|t| t.name == "tick").and_then(|t| t.arg("rate"));
        Ok(Shape {
            app,
            bin: app.note("bin").and_then(|n| n.text("name")).unwrap_or(app.name),
            start: rule(app, "start")?,
            tick,
            input: func(app, "input")?,
            frame: rule(app, "frame")?,
            dump: func(app, "dump")?,
            report: func(app, "report")?,
            controls,
            width: number(window.arg("width"), "window width")?,
            height: number(window.arg("height"), "window height")?,
            title: window.text("title").unwrap_or(app.name),
            rate: number(rate, "tick rate")?,
        })
    }

    /// The start rule's one parameter, if it has one: `--<name> <n>`.
    pub fn level(&self) -> Option<&'static Param> {
        self.start.params.first()
    }

    /// Without the flag: the rule's own default (`None`), or the least value of a range type.
    pub fn default_level(&self) -> Result<Option<Data>, String> {
        match self.level() {
            None => Ok(None),
            Some(p) if p.has_default => Ok(None),
            Some(p) => p.ty.range.map(|(lo, _)| Some(Data::Int(lo))).ok_or_else(|| format!("missing --{}", p.name)),
        }
    }
}
