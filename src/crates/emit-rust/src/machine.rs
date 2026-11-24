//! `machine.rs`: the world (every state), the machine that holds it with the handlers' state, atomic steps,
//! each handler operation, and what traces are made of (events, replies, pools of drawn values).

use crate::code::{Code, Fx};
use crate::cx::{Cx, Mode};
use crate::dty::{gen_fn, pool_types};
use crate::names::{camel, ident, local};
use spec_lang_ir::HandlerId;
use spec_lang_lir::{LTy, Lir};
use std::collections::BTreeSet;

/// The function performing operation `op` of handler `h`.
pub fn op_fn(lir: &Lir, h: HandlerId, op: u32) -> String {
    let handler = lir.handler(h);
    let name = handler.ops.get(op as usize).map_or("op", |o| o.name.as_str());
    ident(&format!("{}_{}", handler.name, name))
}

fn handler_field(lir: &Lir, h: usize) -> String {
    let name = &lir.handlers[h].name;
    if lir.handlers.iter().filter(|x| x.name == *name).count() > 1 { ident(&format!("{name}_{h}")) } else { ident(name) }
}

fn handler_state(lir: &Lir, h: usize) -> Option<String> {
    let name = camel(&lir.handlers[h].pretend.as_ref()?.name);
    let shared = lir.handlers.iter().filter(|x| x.pretend.as_ref().is_some_and(|p| camel(&p.name) == name)).count() > 1;
    Some(if shared { format!("{name}{h}") } else { name })
}

pub fn file(cx: &mut Cx<'_>) -> String {
    let lir = cx.lir;
    let mut out = Vec::new();

    // The world.
    let fields: Vec<String> = lir
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            format!("/// `{}`\npub {}: {},", s.info.qualified, cx.names.state(spec_lang_ir::StateId(i as u32)), cx.ty(&s.ty, None))
        })
        .collect();
    out.push(format!(
        "/// Every state's value at one moment.\n#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]\npub struct World {{ {} }}",
        fields.join(" ")
    ));
    let shown: Vec<String> = lir
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| format!("format!(\"{}: {{}}\", self.{}.show())", s.info.qualified, cx.names.state(spec_lang_ir::StateId(i as u32))))
        .collect();
    out.push(format!(
        "impl Show for World {{ fn show(&self) -> String {{ rt::show::seq(\"World(\", [{}].into_iter(), \")\") }} }}",
        shown.join(", ")
    ));

    // Handlers' pretend state.
    let mut hfields = Vec::new();
    let mut hinits = Vec::new();
    for (i, h) in lir.handlers.iter().enumerate() {
        let (Some(p), Some(sname)) = (&h.pretend, handler_state(lir, i)) else { continue };
        let fs: Vec<String> = p.state.iter().map(|s| format!("pub {}: {},", ident(&s.name), cx.ty(&s.ty, None))).collect();
        out.push(format!(
            "/// The state of `{}`, what tests run for `{}`.\n#[derive(Clone, Debug)]\npub struct {sname} {{ {} }}",
            p.name,
            h.name,
            fs.join(" ")
        ));
        let mut fx = Fx::new(cx, None, false);
        let mut inits = Vec::new();
        for s in &p.state {
            let v = match &s.default {
                Some(e) => fx.value(e).seal(),
                None => empty_of(&mut fx, &s.ty).unwrap_or_else(|| "Default::default()".into()),
            };
            inits.push(format!("{}: {v}", ident(&s.name)));
        }
        hfields.push(format!("pub {}: {sname},", handler_field(lir, i)));
        hinits.push(format!("{}: {sname} {{ {} }}", handler_field(lir, i), inits.join(", ")));
    }
    out.push(format!("/// Every handler's pretend state.\n#[derive(Clone, Debug)]\npub struct Handlers {{ {} }}", hfields.join(" ")));

    // The machine.
    let mut fx = Fx::new(cx, None, false);
    let mut lines = Vec::new();
    let mut inits = Vec::new();
    for (i, s) in lir.states.iter().enumerate() {
        let f = fx.cx.names.state(spec_lang_ir::StateId(i as u32)).to_string();
        let v = match &s.init {
            Some(e) => {
                let c = fx.value(e);
                if c.m {
                    fx.cx.unsupported(s.info.span, "an initial state value that needs the machine (a rule, a state or an effect)");
                }
                c.seal()
            }
            None => match empty_of(&mut fx, &s.ty) {
                Some(v) => v,
                None => {
                    fx.cx.unsupported(s.info.span, "a state that is not a collection or an Option needs an initial value");
                    "Default::default()".into()
                }
            },
        };
        lines.push(format!("let {f} = {v};"));
        inits.push(f);
    }
    for (i, s) in lir.states.iter().enumerate() {
        let f = fx.cx.names.state(spec_lang_ir::StateId(i as u32)).to_string();
        if fx.cx.dty_m(&s.check) {
            fx.cx.unsupported(s.info.span, "a state whose type's claim needs the machine");
        }
        if let Some(line) = fx.check(&s.check, &f, &format!("the initial value of {}", s.info.qualified)) {
            lines.push(line);
        }
    }
    out.push(format!(
        "/// One run of the program: the world, the handlers' state.\n#[derive(Clone, Debug)]\npub struct Machine {{ pub world: World, pub handlers: Handlers, }}\n\n\
         impl Machine {{\n\
         /// The initial world: every state's initial value, checked against its type.\n\
         pub fn new() -> R<Machine> {{ {} Ok(Machine {{ world: World {{ {} }}, handlers: Handlers {{ {} }} }}) }}\n\n\
         /// One atomic step: on any halt, the world and the handlers' state go back to what they were.\n\
         pub fn atomic<T>(&mut self, step: impl FnOnce(&mut Machine) -> R<T>) -> R<T> {{\n\
         let saved = (self.world.clone(), self.handlers.clone());\n\
         let result = step(self);\n\
         if result.is_err() {{ (self.world, self.handlers) = saved; }}\n\
         result }}\n}}",
        lines.join(" "),
        inits.join(", "),
        hinits.join(", ")
    ));

    // Handler operations.
    for (hi, h) in lir.handlers.iter().enumerate() {
        for (oi, op) in h.ops.iter().enumerate() {
            out.push(op_item(cx, hi, oi, &op.name));
        }
    }

    // Events, replies.
    let rules: BTreeSet<usize> = lir.rules.iter().enumerate().map(|(i, _)| i).collect();
    let mut variants = Vec::new();
    let mut shows = Vec::new();
    let mut calls = Vec::new();
    for &i in &rules {
        let r = &lir.rules[i];
        let v = cx.names.event(spec_lang_ir::RuleId(i as u32)).to_string();
        let fs: Vec<String> = r.params.iter().map(|p| format!("{}: {}", ident(&p.name), cx.ty(&p.ty, None))).collect();
        variants.push(format!("/// `{}`\n{v} {{ {} }},", r.info.qualified, fs.join(", ")));
        let names: Vec<String> = r.params.iter().map(|p| ident(&p.name)).collect();
        let module = lir.modules[r.info.module.index()].name.rsplit('.').next().unwrap_or("").to_string();
        let items: Vec<String> = r
            .params
            .iter()
            .enumerate()
            .map(|(k, p)| if k == 0 { format!("{}.show()", ident(&p.name)) } else { format!("format!(\"{}: {{}}\", {}.show())", p.name, ident(&p.name)) })
            .collect();
        shows.push(format!(
            "Event::{v} {{ {} }} => rt::show::seq(\"{module}.{}(\", [{}].into_iter(), \")\"),",
            names.join(", "),
            r.info.name,
            items.join(", ")
        ));
        let args: Vec<String> = r
            .params
            .iter()
            .map(|p| {
                let a = if cx.copy(&p.ty) { format!("*{}", ident(&p.name)) } else { format!("{}.clone()", ident(&p.name)) };
                if p.default.is_some() { format!("Some({a})") } else { a }
            })
            .collect();
        let path = cx.names.path(None, r.info.module, &ident(&r.info.name));
        let out_v = cx.names.out(&r.result).unwrap_or("Unit").to_string();
        let mut list = vec!["m".to_string()];
        list.extend(args);
        calls.push(format!(
            "Event::{v} {{ {} }} => match rt::settle({path}({}))? {{ Ok(v) => Reply::Ok(Out::{out_v}(v)), Err(f) => Reply::Err(f) }},",
            names.join(", "),
            list.join(", ")
        ));
    }
    if variants.is_empty() {
        out.push("/// A call of a rule, as a trace records it: this program has no rules.\n#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]\npub enum Event {}".into());
        out.push("impl Show for Event { fn show(&self) -> String { match *self {} } }".into());
    } else {
        out.push(format!(
            "/// A call of a rule with its arguments, as a trace records it.\n#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]\npub enum Event {{ {} }}",
            variants.join(" ")
        ));
        out.push(format!("impl Show for Event {{ fn show(&self) -> String {{ match self {{ {} }} }} }}", shows.join(" ")));
    }
    // Random events, for the rules laws trace.
    let mut drawn: BTreeSet<usize> = BTreeSet::new();
    for l in &lir.laws {
        drawn.extend(l.traced.iter().map(|r| r.index()));
        drawn.extend(l.start_rules.iter().map(|r| r.index()));
    }
    let mut arms = Vec::new();
    for &i in &drawn {
        let r = &lir.rules[i];
        let v = cx.names.event(spec_lang_ir::RuleId(i as u32)).to_string();
        let fs: Vec<String> = r
            .params
            .iter()
            .map(|p| format!("{}: crate::draw::{}(m, rng, pools)?", ident(&p.name), gen_fn(cx, &p.check)))
            .collect();
        arms.push(format!("{i} => Event::{v} {{ {} }},", fs.join(", ")));
    }
    arms.push("_ => return Err(rt::stop(\"E-GEN\", \"no such rule\")),".into());
    let call_body = if calls.is_empty() { "match *self {}".to_string() } else { format!("Ok(match self {{ {} }})", calls.join(" ")) };
    out.push(format!(
        "impl Event {{\n\
         /// A call of rule number `rule` with arguments drawn from its parameters' types.\n\
         #[allow(unused_variables)]\n\
         pub fn random(m: &mut Machine, rng: &mut Rng, pools: &Pools, rule: usize) -> R<Event> {{ Ok(match rule {{ {} }}) }}\n\n\
         /// The call as one step from outside: its value or its refusal.\n\
         pub fn call(&self, m: &mut Machine) -> R<Reply> {{ {call_body} }}\n}}",
        arms.join(" ")
    ));
    let outs: Vec<(String, String)> = cx.names.outs().map(|(t, n)| (n.clone(), cx.ty(t, None))).collect();
    let ovars: Vec<String> = outs.iter().map(|(n, t)| format!("{n}({t}),")).collect();
    let oshow: Vec<String> = outs.iter().map(|(n, _)| format!("Out::{n}(v) => v.show(),")).collect();
    if outs.is_empty() {
        out.push("/// What a rule gives: this program has no rules.\n#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]\npub enum Out { Unit(()) }".into());
        out.push("impl Show for Out { fn show(&self) -> String { match self { Out::Unit(v) => v.show() } } }".into());
    } else {
        out.push(format!(
            "/// What a rule gives, one variant per result type.\n#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]\npub enum Out {{ {} }}",
            ovars.join(" ")
        ));
        out.push(format!("impl Show for Out {{ fn show(&self) -> String {{ match self {{ {} }} }} }}", oshow.join(" ")));
    }
    out.push(
        "/// A step's reply: the rule's value or its refusal.\n#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]\npub enum Reply { Ok(Out), Err(Failure) }".into(),
    );
    out.push("impl Show for Reply { fn show(&self) -> String { match self { Reply::Ok(v) => format!(\"Ok({})\", v.show()), Reply::Err(f) => format!(\"Err({})\", f.show()) } } }".into());

    // Pools.
    let pools = pool_types(cx);
    let pfields: Vec<String> = pools.iter().enumerate().map(|(i, d)| format!("pub p{i}: Vec<{}>,", cx.ty(&cx.lty(d), None))).collect();
    let mut fills = Vec::new();
    for (si, s) in lir.states.iter().enumerate() {
        let f = cx.names.state(spec_lang_ir::StateId(si as u32));
        match &s.check {
            spec_lang_lir::DTy::Map(k, _) => {
                let p = pools.iter().position(|d| d == &**k).unwrap_or(0);
                fills.push(format!("p{p}.extend(w.{f}.keys().cloned());"));
            }
            spec_lang_lir::DTy::Set(k) => {
                let p = pools.iter().position(|d| d == &**k).unwrap_or(0);
                fills.push(format!("p{p}.extend(w.{f}.iter().cloned());"));
            }
            _ => {}
        }
    }
    let decls: Vec<String> = (0..pools.len()).map(|i| format!("let mut p{i} = BTreeSet::new();")).collect();
    let builds: Vec<String> = (0..pools.len()).map(|i| format!("p{i}: p{i}.into_iter().collect()")).collect();
    out.push(format!(
        "/// Values already in a world, by declared type: the keys of each state declared `Map[K, V]` and the elements of each `Set[K]`.\n\
         #[derive(Clone, Debug, Default)]\npub struct Pools {{ {} }}\n\n\
         impl Pools {{\n#[allow(unused_variables)]\npub fn of(worlds: &[World]) -> Pools {{ {} for w in worlds {{ {} }} Pools {{ {} }} }}\n}}",
        pfields.join(" "),
        decls.join(" "),
        fills.join(" "),
        builds.join(", ")
    ));
    let wf: Vec<String> = lir
        .states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let f = cx.names.state(spec_lang_ir::StateId(i as u32)).to_string();
            format!("{f}: crate::draw::{}(m, rng, &pools)?", gen_fn(cx, &s.check))
        })
        .collect();
    if lir.laws.is_empty() {
        out.push("/// A world drawn from the states' declared types: no law asks for one here.\npub fn random_world(_: &mut Machine, _: &mut Rng) -> R<World> { Err(rt::stop(\"E-GEN\", \"no law draws a world\")) }".into());
    } else {
        out.push(format!(
            "/// A world drawn from the states' declared types, state by state.\npub fn random_world(m: &mut Machine, rng: &mut Rng) -> R<World> {{ let pools = Pools::default(); Ok(World {{ {} }}) }}",
            wf.join(", ")
        ));
    }
    out.join("\n\n")
}

/// The value a state or handler state starts with when none is written.
fn empty_of(fx: &mut Fx<'_, '_>, t: &LTy) -> Option<String> {
    match t {
        LTy::Map(..) => Some("BTreeMap::new()".into()),
        LTy::Set(_) => Some("BTreeSet::new()".into()),
        LTy::List(_) => Some("Vec::new()".into()),
        LTy::Option(_) => Some("Opt::None".into()),
        _ => {
            let _ = fx;
            None
        }
    }
}

fn op_item(cx: &mut Cx<'_>, hi: usize, oi: usize, name: &str) -> String {
    let lir = cx.lir;
    let h = &lir.handlers[hi];
    let op = &h.ops[oi];
    let f = op_fn(lir, HandlerId(hi as u32), oi as u32);
    let ret = cx.ty(&op.result, None);
    let arm = h.pretend.as_ref().and_then(|p| p.arms.get(oi).and_then(Option::as_ref).map(|a| (p, a)));
    let doc = format!("/// `{}.{name}` of library `{}`: in checks, its pretend runs.", h.name, h.library);
    let Some((pretend, arm)) = arm else {
        let params: Vec<String> = op
            .params
            .iter()
            .flatten()
            .map(|p| format!("_{}: {}", ident(&p.name), cx.ty(&p.ty, None)))
            .collect();
        return format!(
            "{doc}\npub fn {f}(_m: &mut Machine, {}) -> R<{ret}> {{ Err(rt::stop(\"E-UNSUPPORTED\", \"`{}.{name}`: library `{}` has no pretend for it\")) }}",
            params.join(", "),
            h.name,
            h.library
        );
    };
    let params: Vec<String> = arm.params.iter().map(|(n, t)| format!("{}: {}", local(n), cx.ty(t, None))).collect();
    let field = handler_field(lir, hi);
    let mut fx = Fx::new(cx, None, true);
    let mut assigned = BTreeSet::new();
    crate::stmt::assigned_in_expr(&arm.body, &mut assigned);
    fx.assigned = assigned.clone();
    let mut lines = Vec::new();
    for ((n, _), p) in arm.params.iter().zip(op.params.iter().flatten()) {
        if let Some(line) = fx.check(&p.check, &local(n), &format!("argument `{}` of `{}.{name}`", p.name, h.name)) {
            lines.push(line);
        }
    }
    for s in &pretend.state {
        let mt = if assigned.contains(&s.name) { "mut " } else { "" };
        lines.push(format!("let {mt}{} = m.handlers.{field}.{}.clone();", local(&s.name), ident(&s.name)));
    }
    let v: Code = fx.value(&arm.body);
    lines.extend(v.pre.iter().cloned());
    if arm.body.ty == op.result {
        lines.push(format!("let reply: {ret} = {};", v.text));
    } else {
        // A rule operation's body runs for its effect on the state; its reply is `()`.
        lines.push(format!("let _ = {};", v.text));
        lines.push(format!("let reply: {ret} = Default::default();"));
    }
    for s in &pretend.state {
        if assigned.contains(&s.name) {
            lines.push(format!("m.handlers.{field}.{} = {};", ident(&s.name), local(&s.name)));
        }
    }
    lines.push("Ok(reply)".into());
    let _ = Mode::Tests;
    format!("{doc}\npub fn {f}(m: &mut Machine, {}) -> R<{ret}> {{ {} }}", params.join(", "), lines.join(" "))
}
