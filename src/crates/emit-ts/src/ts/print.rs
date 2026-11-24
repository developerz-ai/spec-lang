//! swc tree to text: one module item at a time, a blank line between
//! declarations, comments attached by position.

use swc_common::comments::{Comment, CommentKind, Comments, SingleThreadedComments};
use swc_common::sync::Lrc;
use swc_common::{BytePos, DUMMY_SP, SourceMap, Span};
use swc_ecma_ast::{EsVersion, Module, ModuleDecl, ModuleItem};
use swc_ecma_codegen::text_writer::JsWriter;
use swc_ecma_codegen::{Config, Emitter};

/// Hands out positions for nodes that carry a leading comment, and keeps the comments.
#[derive(Default)]
pub struct Notes {
    comments: SingleThreadedComments,
    next: u32,
}

impl std::fmt::Debug for Notes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Notes").field("next", &self.next).finish()
    }
}

impl Notes {
    /// A fresh position with these comments in front of it: `//` lines, then `/** */` blocks.
    pub fn span(&mut self, lines: &[String], docs: &[String]) -> Span {
        self.next += 1;
        let pos = BytePos(self.next);
        for l in lines {
            self.comments
                .add_leading(pos, Comment { kind: CommentKind::Line, span: DUMMY_SP, text: format!(" {l}").into() });
        }
        for d in docs {
            let text = d.replace("*/", "* /");
            self.comments.add_leading(
                pos,
                Comment { kind: CommentKind::Block, span: DUMMY_SP, text: format!("* {text} ").into() },
            );
        }
        Span::new(pos, pos)
    }
}

/// Print a module: the header comment, the imports, then each item after a blank line.
pub fn print(header: &str, items: Vec<ModuleItem>, notes: &Notes) -> Result<String, std::io::Error> {
    let cm: Lrc<SourceMap> = Default::default();
    let mut out = format!("// {header}\n");
    let mut in_imports = false;
    for item in items {
        let is_import = matches!(item, ModuleItem::ModuleDecl(ModuleDecl::Import(_)));
        if !(is_import && in_imports) {
            out.push('\n');
        }
        in_imports = is_import;
        let mut buf = Vec::new();
        {
            let wr = JsWriter::new(cm.clone(), "\n", &mut buf, None);
            let mut emitter = Emitter {
                cfg: Config::default().with_target(EsVersion::EsNext),
                cm: cm.clone(),
                comments: Some(&notes.comments),
                wr,
            };
            emitter.emit_module(&Module { span: DUMMY_SP, body: vec![item], shebang: None })?;
        }
        out.push_str(String::from_utf8_lossy(&buf).trim_end());
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ts::build as b;
    use swc_ecma_ast::{BinaryOp, TsKeywordTypeKind};

    #[test]
    fn a_function_with_comments_and_types() {
        let mut notes = Notes::default();
        let guard = notes.span(&["slg: m.slg:2:1 m.f guard[0]".into()], &[]);
        let body = vec![
            b::at(
                b::if_return(
                    b::not(b::method(b::member(b::name("w"), "tasks"), "has", vec![b::name("no")])),
                    Some(b::text("NoTask")),
                ),
                guard,
            ),
            b::const_(
                "xs",
                Some(b::t_list(b::t_union(vec![
                    b::t_ref("Tag", vec![]),
                    b::keyword(TsKeywordTypeKind::TsUndefinedKeyword),
                ]))),
                b::array(vec![]),
            ),
            b::ret(Some(b::template(
                vec!["a ".into(), " `b`".into()],
                vec![b::bin(BinaryOp::Add, b::big(1), b::num(2.0))],
            ))),
        ];
        let f = b::function(
            "f",
            vec![
                b::param("w", b::t_ref("World", vec![])),
                b::param_default("n", b::keyword(TsKeywordTypeKind::TsBigIntKeyword), b::big(1)),
            ],
            Some(b::keyword(TsKeywordTypeKind::TsStringKeyword)),
            body,
        );
        let top = notes.span(&["slg: m.slg:1:1 m.f fn".into()], &["why".into()]);
        let items = vec![
            b::import("./w.ts", vec![("World".into(), true), ("x".into(), false)]),
            b::import("./t.ts", vec![("Tag".into(), true)]),
            b::export(f, top),
        ];
        let text = print("header", items, &notes).unwrap();
        let want = "// header\n\nimport { type World, x } from \"./w.ts\";\nimport type { Tag } from \"./t.ts\";\n\n// slg: m.slg:1:1 m.f fn\n/** why */ export function f(w: World, n: bigint = 1n): string {\n    // slg: m.slg:2:1 m.f guard[0]\n    if (!w.tasks.has(no)) return \"NoTask\";\n    const xs: ReadonlyArray<Tag | undefined> = [];\n    return `a ${1n + 2} \\`b\\``;\n}\n";
        assert_eq!(text, want);
    }
}
