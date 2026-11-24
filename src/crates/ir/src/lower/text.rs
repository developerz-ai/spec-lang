//! Text literals: escapes and `{expr}` interpolation.

use super::Lowerer;
use super::expr::{Cx, R};
use crate::error::IrError;
use crate::expr::*;
use spec_lang_syntax::ast::TextLit;
use spec_lang_syntax::{Span, parse_expr};

impl Lowerer {
    /// Escapes `\n \t \\ \" \{ \u{...}`; `{expr}` interpolates (any data prints).
    pub(crate) fn lower_text(&mut self, cx: &mut Cx, t: &TextLit) -> R<Expr> {
        if let Some(p) = &t.prefix {
            return Err(IrError::unsupported(t.span, format!("the `{p}\"...\"` literal")));
        }
        let file = t.span.file;
        let src = self.prog.sources.get(file).map(|f| f.text.clone()).unwrap_or_default();
        let at = |i: usize| t.raw_offset as usize + i;
        let mut parts = Vec::new();
        let mut lit = String::new();
        let raw = &t.raw;
        let mut i = 0;
        while i < raw.len() {
            let rest = &raw[i..];
            let c = rest.chars().next().unwrap_or('\0');
            if c == '\\' {
                let (ch, len) = escape(rest).ok_or_else(|| {
                    let s = Span::new(file, u32::try_from(at(i)).unwrap_or(0), u32::try_from(at(i + 2)).unwrap_or(0));
                    IrError::unsupported(s, "this escape: `\\n \\t \\\\ \\\" \\{ \\u{...}`")
                })?;
                lit.push(ch);
                i += len;
            } else if c == '{' {
                let close = rest.find('}').ok_or_else(|| {
                    IrError::error(
                        "E-PARSE",
                        t.span,
                        "`{` opens an interpolation that never closes; write `\\{` for a brace",
                    )
                })?;
                if !lit.is_empty() {
                    parts.push(InterpPart::Lit(std::mem::take(&mut lit)));
                }
                let (lo, hi) = (at(i + 1), at(i + close));
                if rest[1..close].contains('\\') {
                    return Err(IrError::unsupported(t.span, "a backslash inside an interpolation"));
                }
                let e = parse_expr(file, &src, lo, hi).map_err(|d| IrError::syntax(&d))?;
                parts.push(InterpPart::Expr(self.lower_expr(cx, &e)?));
                i += close + 1;
            } else {
                lit.push(c);
                i += c.len_utf8();
            }
        }
        if parts.is_empty() {
            return Ok(Expr::new(ExprKind::Text(lit), t.span));
        }
        if !lit.is_empty() {
            parts.push(InterpPart::Lit(lit));
        }
        Ok(Expr::new(ExprKind::Interp(parts), t.span))
    }
}

/// One escape at the start of `s` (which starts with `\`): the character and the bytes used.
fn escape(s: &str) -> Option<(char, usize)> {
    let mut it = s.chars();
    it.next();
    match it.next()? {
        'n' => Some(('\n', 2)),
        't' => Some(('\t', 2)),
        '\\' => Some(('\\', 2)),
        '"' => Some(('"', 2)),
        '{' => Some(('{', 2)),
        'u' => {
            let body = s.get(2..)?.strip_prefix('{')?;
            let end = body.find('}')?;
            let n = u32::from_str_radix(body.get(..end)?, 16).ok()?;
            Some((char::from_u32(n)?, 4 + end))
        }
        _ => None,
    }
}
