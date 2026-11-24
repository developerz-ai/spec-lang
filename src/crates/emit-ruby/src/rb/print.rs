//! The Ruby tree to text. Two spaces, double quotes, one statement per line;
//! a list too long for a line breaks one item per line.

use super::ast::*;

const WIDTH: usize = 110;

fn escape(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '#' => out.push_str("\\#"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => out.push_str(&format!("\\u{{{:04X}}}", c as u32)),
            c => out.push(c),
        }
    }
}

fn plain_symbol(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && cs.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn atomic(e: &Expr) -> bool {
    !matches!(e, Expr::Bin(..) | Expr::Ternary(..) | Expr::Not(_) | Expr::Neg(_) | Expr::Lambda(..))
}

fn wrapped(e: &Expr, indent: usize) -> String {
    if atomic(e) { expr(e, indent) } else { format!("({})", expr(e, indent)) }
}

fn pad(n: usize) -> String {
    "  ".repeat(n)
}

/// Items on one line, or one per line when that would be too long.
fn list(open: &str, items: Vec<String>, close: &str, indent: usize, spaced: bool) -> String {
    let flat = if items.is_empty() {
        format!("{open}{close}")
    } else if spaced {
        format!("{open} {} {close}", items.join(", "))
    } else {
        format!("{open}{}{close}", items.join(", "))
    };
    if flat.len() + indent * 2 <= WIDTH || items.len() < 2 || flat.contains('\n') && items.len() < 2 {
        return flat;
    }
    let inner = pad(indent + 1);
    format!("{open}\n{inner}{}\n{}{close}", items.join(&format!(",\n{inner}")), pad(indent))
}

fn param(p: &Param, indent: usize) -> String {
    match p {
        Param::Req(n) => n.clone(),
        Param::Key(n) => format!("{n}:"),
        Param::KeyOpt(n, d) => format!("{n}: {}", expr(d, indent)),
        Param::Rest2(n) => format!("**{n}"),
    }
}

fn arg(a: &Arg, indent: usize) -> String {
    match a {
        Arg::Pos(e) => expr(e, indent),
        Arg::Key(n, Expr::Ident(v)) if n == v => format!("{n}:"),
        Arg::Key(n, e) => format!("{n}: {}", expr(e, indent)),
        Arg::Splat2(e) => format!("**{}", expr(e, indent)),
    }
}

fn block(b: &Block, indent: usize) -> String {
    let params = if b.params.is_empty() { String::new() } else { format!(" |{}|", b.params.join(", ")) };
    match b.body.as_slice() {
        [Stmt::Expr(e)] => {
            let one = format!(" {{{params} {} }}", expr(e, indent));
            if !one.contains('\n') {
                return one;
            }
        }
        [] => return format!(" {{{params} }}"),
        _ => {}
    }
    let mut out = format!(" do{params}\n");
    stmts(&b.body, indent + 1, &mut out);
    out.push_str(&pad(indent));
    out.push_str("end");
    out
}

pub fn expr(e: &Expr, indent: usize) -> String {
    match e {
        Expr::Nil => "nil".into(),
        Expr::True => "true".into(),
        Expr::False => "false".into(),
        Expr::SelfRef => "self".into(),
        Expr::Int(i) => i.to_string(),
        Expr::Str(s) => {
            let mut out = String::from("\"");
            escape(s, &mut out);
            out.push('"');
            out
        }
        Expr::Sym(s) => {
            if plain_symbol(s) {
                format!(":{s}")
            } else {
                let mut out = String::from(":\"");
                escape(s, &mut out);
                out.push('"');
                out
            }
        }
        Expr::Interp(parts) => {
            let mut out = String::from("\"");
            for p in parts {
                match p {
                    Part::Lit(s) => escape(s, &mut out),
                    Part::Expr(e) => {
                        out.push_str("#{");
                        out.push_str(&expr(e, indent));
                        out.push('}');
                    }
                }
            }
            out.push('"');
            out
        }
        Expr::Ident(n) => n.clone(),
        Expr::Const(path) => path.join("::"),
        Expr::Call { recv, name, args, block: blk } => {
            let mut out = String::new();
            if let Some(r) = recv {
                out.push_str(&wrapped(r, indent));
                out.push('.');
            }
            out.push_str(name);
            if !args.is_empty() {
                out.push_str(&list("(", args.iter().map(|a| arg(a, indent + 1)).collect(), ")", indent, false));
            }
            if let Some(b) = blk {
                out.push_str(&block(b, indent));
            }
            out
        }
        Expr::Index(recv, key) => format!("{}[{}]", wrapped(recv, indent), expr(key, indent)),
        Expr::Array(items) => list("[", items.iter().map(|i| expr(i, indent + 1)).collect(), "]", indent, false),
        Expr::SetLit(items) => list("Set[", items.iter().map(|i| expr(i, indent + 1)).collect(), "]", indent, false),
        Expr::Hash(pairs) => {
            let items = pairs
                .iter()
                .map(|(k, v)| match k {
                    Expr::Sym(s) if plain_symbol(s) => format!("{s}: {}", expr(v, indent + 1)),
                    k => format!("{} => {}", expr(k, indent + 1), expr(v, indent + 1)),
                })
                .collect();
            list("{", items, "}", indent, true)
        }
        Expr::Not(x) => format!("!{}", wrapped(x, indent)),
        Expr::Neg(x) => format!("-{}", wrapped(x, indent)),
        Expr::Bin(op, a, b) => {
            // A chain of one logical operator needs no parentheses.
            let side = |x: &Expr| match x {
                Expr::Bin(inner, ..) if inner == op && matches!(*op, "&&" | "||") => expr(x, indent),
                x => wrapped(x, indent),
            };
            format!("{} {op} {}", side(a), side(b))
        }
        Expr::Ternary(c, a, b) => format!("{} ? {} : {}", wrapped(c, indent), wrapped(a, indent), wrapped(b, indent)),
        Expr::Lambda(params, body) => {
            let ps: Vec<String> = params.iter().map(|p| param(p, indent)).collect();
            format!("->({}) {{ {} }}", ps.join(", "), expr(body, indent))
        }
    }
}

fn body(name_line: &str, inner: &[Stmt], indent: usize, out: &mut String) {
    out.push_str(&pad(indent));
    out.push_str(name_line);
    out.push('\n');
    stmts(inner, indent + 1, out);
    out.push_str(&pad(indent));
    out.push_str("end\n");
}

fn simple(s: &Stmt, indent: usize) -> Option<String> {
    Some(match s {
        Stmt::Expr(e) => expr(e, indent),
        Stmt::Assign(n, e) => format!("{n} = {}", expr(e, indent)),
        Stmt::Return(None) => "return".into(),
        Stmt::Return(Some(e)) => format!("return {}", expr(e, indent)),
        Stmt::Raise(e) => format!("raise {}", expr(e, indent)),
        Stmt::Super(args) => format!("super({})", args.iter().map(|a| arg(a, indent)).collect::<Vec<_>>().join(", ")),
        _ => return None,
    })
}

pub fn stmts(list: &[Stmt], indent: usize, out: &mut String) {
    for s in list {
        stmt(s, indent, out);
    }
}

pub fn stmt(s: &Stmt, indent: usize, out: &mut String) {
    if let Some(line) = simple(s, indent) {
        out.push_str(&pad(indent));
        out.push_str(&line);
        out.push('\n');
        return;
    }
    match s {
        Stmt::ConstAssign(n, e) => {
            out.push_str(&format!("{}{n} = {}\n", pad(indent), expr(e, indent)));
        }
        Stmt::If { cond, then, els } => {
            out.push_str(&format!("{}if {}\n", pad(indent), expr(cond, indent)));
            stmts(then, indent + 1, out);
            if let Some(els) = els {
                out.push_str(&format!("{}else\n", pad(indent)));
                stmts(els, indent + 1, out);
            }
            out.push_str(&format!("{}end\n", pad(indent)));
        }
        Stmt::Modifier { stmt: inner, keyword, cond } => {
            let line = simple(inner, indent).unwrap_or_default();
            out.push_str(&format!("{}{line} {keyword} {}\n", pad(indent), expr(cond, indent)));
        }
        Stmt::While { cond, body: inner } => body(&format!("while {}", expr(cond, indent)), inner, indent, out),
        Stmt::Def { singleton, name, params, body: inner } => {
            let ps: Vec<String> = params.iter().map(|p| param(p, indent)).collect();
            let ps = if ps.is_empty() { String::new() } else { format!("({})", ps.join(", ")) };
            body(&format!("def {}{name}{ps}", if *singleton { "self." } else { "" }), inner, indent, out);
        }
        Stmt::Module { name, body: inner } => body(&format!("module {name}"), inner, indent, out),
        Stmt::Class { name, superclass, body: inner } => {
            let sup = superclass.as_ref().map(|s| format!(" < {}", expr(s, indent))).unwrap_or_default();
            if inner.is_empty() {
                out.push_str(&format!("{}class {name}{sup}; end\n", pad(indent)));
            } else {
                body(&format!("class {name}{sup}"), inner, indent, out);
            }
        }
        Stmt::DataDefine { name, fields, body: inner } => {
            let fs: Vec<String> = fields.iter().map(|f| format!(":{f}")).collect();
            let head = format!("{name} = Data.define({})", fs.join(", "));
            if inner.is_empty() {
                out.push_str(&format!("{}{head}\n", pad(indent)));
            } else {
                body(&format!("{head} do"), inner, indent, out);
            }
        }
        Stmt::Case { on, whens } => {
            out.push_str(&format!("{}case {}\n", pad(indent), expr(on, indent)));
            for (when, then) in whens {
                out.push_str(&format!("{}when {} then {}\n", pad(indent), expr(when, indent), expr(then, indent)));
            }
            out.push_str(&format!("{}end\n", pad(indent)));
        }
        Stmt::Comment(text) => out.push_str(&format!("{}# {text}\n", pad(indent))),
        Stmt::Blank => out.push('\n'),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rule_prints_as_ruby() {
        let guard = Stmt::Modifier {
            stmt: Box::new(Stmt::Raise(konst("App::NoTask"))),
            keyword: "unless",
            cond: call(call(ident("world"), "tasks", vec![]), "key?", vec![ident("no")]),
        };
        let line = Expr::Interp(vec![Part::Lit("removed #".into()), Part::Expr(ident("no"))]);
        let def = Stmt::Def {
            singleton: true,
            name: "remove".into(),
            params: vec![Param::Req("world".into()), Param::Key("no".into())],
            body: vec![
                Stmt::Comment("slg: m.slg:1:1 m.r guard[0]".into()),
                guard,
                Stmt::Assign("lines".into(), Expr::Array(vec![line])),
            ],
        };
        let mut out = String::new();
        stmt(&Stmt::Module { name: "Board".into(), body: vec![def] }, 0, &mut out);
        assert_eq!(
            out,
            "module Board\n  def self.remove(world, no:)\n    # slg: m.slg:1:1 m.r guard[0]\n    raise App::NoTask unless world.tasks.key?(no)\n    lines = [\"removed \\##{no}\"]\n  end\nend\n"
        );
    }

    #[test]
    fn blocks_and_operators() {
        let e = call_block(
            ident("xs"),
            "select",
            vec![],
            &["t"],
            vec![Stmt::Expr(bin("&&", bin("||", ident("all"), bin("==", ident("s"), sym("Open"))), ident("ok")))],
        );
        assert_eq!(expr(&e, 0), "xs.select { |t| (all || (s == :Open)) && ok }");
        assert_eq!(expr(&not(bin("==", ident("a"), Expr::Nil)), 0), "a != nil");
        assert_eq!(
            expr(&Expr::Hash(vec![(sym("a"), Expr::Int(1)), (text("k"), Expr::SetLit(vec![]))]), 0),
            "{ a: 1, \"k\" => Set[] }"
        );
    }
}
