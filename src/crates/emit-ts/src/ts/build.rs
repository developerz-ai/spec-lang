//! Short constructors for the swc TypeScript tree. Nothing here knows about
//! spec-lang; it only keeps the lowering readable.
//!
//! The swc printer adds no parentheses, so the constructors that take an
//! operand ([`bin`], [`not`], [`member`], [`call`]) wrap anything that is not
//! atomic. The generated project's formatter removes the ones not needed.

use swc_common::{DUMMY_SP, Span, SyntaxContext};
use swc_ecma_ast::*;

pub type E = Box<Expr>;
pub type T = Box<TsType>;

pub fn id(name: &str) -> Ident {
    Ident::new_no_ctxt(name.into(), DUMMY_SP)
}

pub fn name(n: &str) -> E {
    Box::new(Expr::Ident(id(n)))
}

pub fn undefined() -> E {
    name("undefined")
}

pub fn str_lit(s: &str) -> Str {
    Str { span: DUMMY_SP, value: s.into(), raw: None }
}

pub fn text(s: &str) -> E {
    Box::new(Expr::Lit(Lit::Str(str_lit(s))))
}

pub fn num(n: f64) -> E {
    Box::new(Expr::Lit(Lit::Num(Number { span: DUMMY_SP, value: n, raw: None })))
}

pub fn big(n: i128) -> E {
    Box::new(Expr::Lit(Lit::BigInt(BigInt { span: DUMMY_SP, value: Box::new(BigIntValue::from(n)), raw: None })))
}

pub fn boolean(b: bool) -> E {
    Box::new(Expr::Lit(Lit::Bool(Bool { span: DUMMY_SP, value: b })))
}

fn atomic(e: &Expr) -> bool {
    matches!(
        e,
        Expr::Ident(_)
            | Expr::Lit(_)
            | Expr::Member(_)
            | Expr::Call(_)
            | Expr::Paren(_)
            | Expr::Array(_)
            | Expr::Object(_)
            | Expr::Tpl(_)
            | Expr::This(_)
    )
}

pub fn paren(e: E) -> E {
    if atomic(&e) { e } else { Box::new(Expr::Paren(ParenExpr { span: DUMMY_SP, expr: e })) }
}

pub fn member(obj: E, prop: &str) -> E {
    Box::new(Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: paren(obj),
        prop: MemberProp::Ident(IdentName::new(prop.into(), DUMMY_SP)),
    }))
}

fn arg(e: E) -> ExprOrSpread {
    ExprOrSpread { spread: None, expr: e }
}

pub fn call(callee: E, args: Vec<E>) -> E {
    Box::new(Expr::Call(CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Callee::Expr(paren(callee)),
        args: args.into_iter().map(arg).collect(),
        type_args: None,
    }))
}

/// `f<T, U>(args)`.
pub fn call_t(callee: E, types: Vec<T>, args: Vec<E>) -> E {
    Box::new(Expr::Call(CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Callee::Expr(paren(callee)),
        args: args.into_iter().map(arg).collect(),
        type_args: Some(Box::new(TsTypeParamInstantiation { span: DUMMY_SP, params: types })),
    }))
}

/// `obj.method(args)`.
pub fn method(obj: E, m: &str, args: Vec<E>) -> E {
    call(member(obj, m), args)
}

pub fn new(class: &str, types: Vec<T>, args: Vec<E>) -> E {
    Box::new(Expr::New(NewExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: name(class),
        args: Some(args.into_iter().map(arg).collect()),
        type_args: if types.is_empty() {
            None
        } else {
            Some(Box::new(TsTypeParamInstantiation { span: DUMMY_SP, params: types }))
        },
    }))
}

pub fn bin(op: BinaryOp, a: E, b: E) -> E {
    Box::new(Expr::Bin(BinExpr { span: DUMMY_SP, op, left: paren(a), right: paren(b) }))
}

/// `!e`, folding a comparison or a double negation.
// The builders pass expressions boxed, as swc holds them.
#[allow(clippy::boxed_local)]
pub fn not(e: E) -> E {
    match *e {
        Expr::Unary(UnaryExpr { op: UnaryOp::Bang, arg, .. }) => arg,
        Expr::Bin(BinExpr { op: BinaryOp::EqEqEq, left, right, .. }) => bin(BinaryOp::NotEqEq, left, right),
        Expr::Bin(BinExpr { op: BinaryOp::NotEqEq, left, right, .. }) => bin(BinaryOp::EqEqEq, left, right),
        other => Box::new(Expr::Unary(UnaryExpr { span: DUMMY_SP, op: UnaryOp::Bang, arg: paren(Box::new(other)) })),
    }
}

pub fn neg(e: E) -> E {
    Box::new(Expr::Unary(UnaryExpr { span: DUMMY_SP, op: UnaryOp::Minus, arg: paren(e) }))
}

pub fn cond(test: E, cons: E, alt: E) -> E {
    Box::new(Expr::Cond(CondExpr { span: DUMMY_SP, test: paren(test), cons: paren(cons), alt: paren(alt) }))
}

pub fn array(items: Vec<E>) -> E {
    Box::new(Expr::Array(ArrayLit { span: DUMMY_SP, elems: items.into_iter().map(|e| Some(arg(e))).collect() }))
}

/// `[...e]`.
pub fn spread_array(e: E) -> E {
    Box::new(Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems: vec![Some(ExprOrSpread { spread: Some(DUMMY_SP), expr: e })],
    }))
}

/// An object literal; a property whose value is the identifier of its own name is written short.
pub fn object(props: Vec<(String, E)>) -> E {
    let props = props
        .into_iter()
        .map(|(k, v)| {
            let short = matches!(&*v, Expr::Ident(i) if i.sym.as_str() == k);
            PropOrSpread::Prop(Box::new(if short {
                Prop::Shorthand(id(&k))
            } else {
                Prop::KeyValue(KeyValueProp {
                    key: PropName::Ident(IdentName::new(k.as_str().into(), DUMMY_SP)),
                    value: v,
                })
            }))
        })
        .collect();
    Box::new(Expr::Object(ObjectLit { span: DUMMY_SP, props }))
}

/// A template literal from alternating texts and expressions; `quasis` has one more item than `exprs`.
pub fn template(quasis: Vec<String>, exprs: Vec<E>) -> E {
    let last = quasis.len().saturating_sub(1);
    let quasis = quasis
        .into_iter()
        .enumerate()
        .map(|(i, q)| {
            let raw = q
                .replace('\\', "\\\\")
                .replace('`', "\\`")
                .replace("${", "\\${")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            TplElement { span: DUMMY_SP, tail: i == last, cooked: None, raw: raw.into() }
        })
        .collect();
    Box::new(Expr::Tpl(Tpl { span: DUMMY_SP, exprs, quasis }))
}

pub fn as_const(e: E) -> E {
    Box::new(Expr::TsConstAssertion(TsConstAssertion { span: DUMMY_SP, expr: e }))
}

pub fn param(n: &str, ty: T) -> Pat {
    Pat::Ident(BindingIdent { id: id(n), type_ann: Some(ann(ty)) })
}

pub fn param_default(n: &str, ty: T, default: E) -> Pat {
    Pat::Assign(AssignPat { span: DUMMY_SP, left: Box::new(param(n, ty)), right: default })
}

pub fn bare(n: &str) -> Pat {
    Pat::Ident(BindingIdent { id: id(n), type_ann: None })
}

/// `(a, b) => body`.
// The builders pass expressions boxed, as swc holds them.
#[allow(clippy::boxed_local)]
pub fn arrow(params: Vec<Pat>, body: E) -> E {
    Box::new(Expr::Arrow(ArrowExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        params,
        body: Box::new(ArrowFunctionBody::Expr(match *body {
            b @ Expr::Object(_) => Box::new(Expr::Paren(ParenExpr { span: DUMMY_SP, expr: Box::new(b) })),
            b => Box::new(b),
        })),
        is_async: false,
        is_generator: false,
        type_params: None,
        return_type: None,
    }))
}

/// `(a, b) => { stmts }`.
pub fn arrow_block(params: Vec<Pat>, stmts: Vec<Stmt>) -> E {
    Box::new(Expr::Arrow(ArrowExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        params,
        body: Box::new(ArrowFunctionBody::FunctionBody(FunctionBody { span: DUMMY_SP, stmts })),
        is_async: false,
        is_generator: false,
        type_params: None,
        return_type: None,
    }))
}

// Statements.

pub fn block(stmts: Vec<Stmt>) -> BlockStmt {
    BlockStmt { span: DUMMY_SP, ctxt: SyntaxContext::empty(), stmts }
}

pub fn ret(e: Option<E>) -> Stmt {
    Stmt::Return(ReturnStmt { span: DUMMY_SP, arg: e })
}

pub fn expr_stmt(e: E) -> Stmt {
    Stmt::Expr(ExprStmt { span: DUMMY_SP, expr: e })
}

pub fn if_(test: E, cons: Vec<Stmt>, alt: Option<Vec<Stmt>>) -> Stmt {
    Stmt::If(IfStmt {
        span: DUMMY_SP,
        test,
        cons: Box::new(Stmt::Block(block(cons))),
        alt: alt.map(|a| Box::new(Stmt::Block(block(a)))),
    })
}

/// `if (test) return e;` on one line.
pub fn if_return(test: E, e: Option<E>) -> Stmt {
    Stmt::If(IfStmt { span: DUMMY_SP, test, cons: Box::new(ret(e)), alt: None })
}

fn var(kind: VarDeclKind, n: &str, ty: Option<T>, init: E) -> VarDecl {
    VarDecl {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        kind,
        declare: false,
        decls: vec![VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent { id: id(n), type_ann: ty.map(ann) }),
            init: Some(init),
            definite: false,
        }],
    }
}

pub fn const_(n: &str, ty: Option<T>, init: E) -> Stmt {
    Stmt::Decl(Decl::Var(Box::new(var(VarDeclKind::Const, n, ty, init))))
}

pub fn let_(n: &str, ty: Option<T>, init: E) -> Stmt {
    Stmt::Decl(Decl::Var(Box::new(var(VarDeclKind::Let, n, ty, init))))
}

pub fn assign(n: &str, value: E) -> Stmt {
    expr_stmt(Box::new(Expr::Assign(AssignExpr {
        span: DUMMY_SP,
        op: AssignOp::Assign,
        left: AssignTarget::Simple(SimpleAssignTarget::Ident(BindingIdent { id: id(n), type_ann: None })),
        right: value,
    })))
}

pub fn for_of(n: &str, iter: E, body: Vec<Stmt>) -> Stmt {
    Stmt::ForOf(ForOfStmt {
        span: DUMMY_SP,
        is_await: false,
        left: ForHead::VarDecl(Box::new(VarDecl {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            kind: VarDeclKind::Const,
            declare: false,
            decls: vec![VarDeclarator { span: DUMMY_SP, name: bare(n), init: None, definite: false }],
        })),
        right: iter,
        body: Box::new(Stmt::Block(block(body))),
    })
}

pub fn while_(test: E, body: Vec<Stmt>) -> Stmt {
    Stmt::While(WhileStmt { span: DUMMY_SP, test, body: Box::new(Stmt::Block(block(body))) })
}

/// `case "a": return e;` for each pair, in one `switch`.
pub fn switch_return(on: E, cases: Vec<(E, E)>) -> Stmt {
    Stmt::Switch(SwitchStmt {
        span: DUMMY_SP,
        body_ctxt: SyntaxContext::empty(),
        discriminant: on,
        cases: cases
            .into_iter()
            .map(|(test, value)| SwitchCase { span: DUMMY_SP, test: Some(test), cons: vec![ret(Some(value))] })
            .collect(),
    })
}

/// Put a position on a statement, so a leading comment can be attached to it.
pub fn at(stmt: Stmt, span: Span) -> Stmt {
    match stmt {
        Stmt::Return(mut s) => {
            s.span = span;
            Stmt::Return(s)
        }
        Stmt::Expr(mut s) => {
            s.span = span;
            Stmt::Expr(s)
        }
        Stmt::If(mut s) => {
            s.span = span;
            Stmt::If(s)
        }
        Stmt::ForOf(mut s) => {
            s.span = span;
            Stmt::ForOf(s)
        }
        Stmt::While(mut s) => {
            s.span = span;
            Stmt::While(s)
        }
        Stmt::Switch(mut s) => {
            s.span = span;
            Stmt::Switch(s)
        }
        Stmt::Decl(Decl::Var(mut v)) => {
            v.span = span;
            Stmt::Decl(Decl::Var(v))
        }
        Stmt::Decl(Decl::Fn(mut f)) => {
            f.function.span = span;
            Stmt::Decl(Decl::Fn(f))
        }
        other => other,
    }
}

// Declarations.

pub fn function(n: &str, params: Vec<Pat>, ret_ty: Option<T>, body: Vec<Stmt>) -> Decl {
    Decl::Fn(FnDecl {
        ident: id(n),
        declare: false,
        function: Box::new(Function {
            this_param: None,
            params: params.into_iter().map(|pat| Param { span: DUMMY_SP, decorators: vec![], pat }).collect(),
            decorators: vec![],
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            body: Some(FunctionBody { span: DUMMY_SP, stmts: body }),
            is_generator: false,
            is_async: false,
            type_params: None,
            return_type: ret_ty.map(ann),
        }),
    })
}

pub fn const_decl(n: &str, ty: Option<T>, init: E) -> Decl {
    Decl::Var(Box::new(var(VarDeclKind::Const, n, ty, init)))
}

pub fn type_alias(n: &str, params: Vec<(&str, Option<T>)>, ty: T) -> Decl {
    Decl::TsTypeAlias(Box::new(TsTypeAliasDecl {
        span: DUMMY_SP,
        declare: false,
        id: id(n),
        type_params: if params.is_empty() {
            None
        } else {
            Some(Box::new(TsTypeParamDecl {
                span: DUMMY_SP,
                params: params
                    .into_iter()
                    .map(|(p, default)| TsTypeParam {
                        span: DUMMY_SP,
                        name: id(p),
                        is_in: false,
                        is_out: false,
                        is_const: false,
                        constraint: None,
                        default,
                    })
                    .collect(),
            }))
        },
        type_ann: ty,
    }))
}

/// An interface whose every property is `readonly`.
pub fn interface(n: &str, fields: Vec<(String, T)>) -> Decl {
    Decl::TsInterface(Box::new(TsInterfaceDecl {
        span: DUMMY_SP,
        id: id(n),
        declare: false,
        type_params: None,
        extends: vec![],
        body: TsInterfaceBody { span: DUMMY_SP, body: fields.into_iter().map(|(k, t)| prop_sig(&k, t)).collect() },
    }))
}

fn prop_sig(k: &str, t: T) -> TsTypeElement {
    TsTypeElement::TsPropertySignature(TsPropertySignature {
        span: DUMMY_SP,
        readonly: true,
        key: name(k),
        computed: false,
        optional: false,
        type_ann: Some(ann(t)),
    })
}

pub fn export(decl: Decl, span: Span) -> ModuleItem {
    ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl { span, decl }))
}

pub fn local(decl: Decl, span: Span) -> ModuleItem {
    ModuleItem::Stmt(at(Stmt::Decl(decl), span))
}

// Types.

pub fn ann(t: T) -> Box<TsTypeAnn> {
    Box::new(TsTypeAnn { span: DUMMY_SP, type_ann: t })
}

pub fn keyword(kind: TsKeywordTypeKind) -> T {
    Box::new(TsType::TsKeywordType(TsKeywordType { span: DUMMY_SP, kind }))
}

pub fn t_ref(n: &str, args: Vec<T>) -> T {
    Box::new(TsType::TsTypeRef(TsTypeRef {
        span: DUMMY_SP,
        type_name: TsEntityName::Ident(id(n)),
        type_params: if args.is_empty() {
            None
        } else {
            Some(Box::new(TsTypeParamInstantiation { span: DUMMY_SP, params: args }))
        },
    }))
}

pub fn t_union(types: Vec<T>) -> T {
    Box::new(TsType::TsUnionOrIntersectionType(TsUnionOrIntersectionType::TsUnionType(TsUnionType {
        span: DUMMY_SP,
        types,
    })))
}

pub fn t_str(s: &str) -> T {
    Box::new(TsType::TsLitType(TsLitType { span: DUMMY_SP, lit: TsLit::Str(str_lit(s)) }))
}

/// `readonly T[]`, written `ReadonlyArray<T>` so a union inside needs no parentheses.
pub fn t_list(t: T) -> T {
    t_ref("ReadonlyArray", vec![t])
}

pub fn t_tuple(types: Vec<T>) -> T {
    Box::new(TsType::TsTypeOperator(TsTypeOperator {
        span: DUMMY_SP,
        op: TsTypeOperatorOp::ReadOnly,
        type_ann: Box::new(TsType::TsTupleType(TsTupleType {
            span: DUMMY_SP,
            elem_types: types.into_iter().map(|ty| TsTupleElement { span: DUMMY_SP, label: None, ty }).collect(),
        })),
    }))
}

/// `(typeof NAME)[number]`.
pub fn t_element_of(n: &str) -> T {
    let query = Box::new(TsType::TsTypeQuery(TsTypeQuery {
        span: DUMMY_SP,
        expr_name: TsTypeQueryExpr::TsEntityName(TsEntityName::Ident(id(n))),
        type_args: None,
    }));
    Box::new(TsType::TsIndexedAccessType(TsIndexedAccessType {
        span: DUMMY_SP,
        readonly: false,
        obj_type: Box::new(TsType::TsParenthesizedType(TsParenthesizedType { span: DUMMY_SP, type_ann: query })),
        index_type: keyword(TsKeywordTypeKind::TsNumberKeyword),
    }))
}

/// `{ readonly a: A; readonly b: B }`.
pub fn t_object(fields: Vec<(String, T)>) -> T {
    Box::new(TsType::TsTypeLit(TsTypeLit {
        span: DUMMY_SP,
        members: fields.into_iter().map(|(k, t)| prop_sig(&k, t)).collect(),
    }))
}

pub fn import(path: &str, names: Vec<(String, bool)>) -> ModuleItem {
    let all_types = names.iter().all(|(_, is_type)| *is_type);
    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        span: DUMMY_SP,
        specifiers: names
            .into_iter()
            .map(|(n, is_type)| {
                ImportSpecifier::Named(ImportNamedSpecifier {
                    span: DUMMY_SP,
                    local: id(&n),
                    imported: None,
                    is_type_only: is_type && !all_types,
                })
            })
            .collect(),
        src: Box::new(str_lit(path)),
        type_only: all_types,
        with: None,
        phase: ImportPhase::Evaluation,
    }))
}

/// `import fc from "fast-check";`
pub fn import_default(path: &str, n: &str) -> ModuleItem {
    ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
        span: DUMMY_SP,
        specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier { span: DUMMY_SP, local: id(n) })],
        src: Box::new(str_lit(path)),
        type_only: false,
        with: None,
        phase: ImportPhase::Evaluation,
    }))
}

/// `obj[key]`.
pub fn index(obj: E, key: E) -> E {
    Box::new(Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: paren(obj),
        prop: MemberProp::Computed(ComputedPropName { span: DUMMY_SP, expr: key }),
    }))
}

/// `[a, b]` as a parameter.
pub fn array_pat(names: &[String]) -> Pat {
    Pat::Array(ArrayPat {
        span: DUMMY_SP,
        elems: names.iter().map(|n| Some(bare(n))).collect(),
        optional: false,
        type_ann: None,
    })
}

/// `f(...xs)`.
pub fn call_spread(callee: E, xs: E) -> E {
    Box::new(Expr::Call(CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: Callee::Expr(paren(callee)),
        args: vec![ExprOrSpread { spread: Some(DUMMY_SP), expr: xs }],
        type_args: None,
    }))
}

/// `{ ...base, k: v }`.
pub fn object_spread(base: E, props: Vec<(String, E)>) -> E {
    let mut out = vec![PropOrSpread::Spread(SpreadElement { dot3_token: DUMMY_SP, expr: base })];
    if let Expr::Object(o) = *object(props) {
        out.extend(o.props);
    }
    Box::new(Expr::Object(ObjectLit { span: DUMMY_SP, props: out }))
}

/// `target = value;` for a member target.
pub fn assign_member(obj: E, prop: &str, value: E) -> Stmt {
    expr_stmt(Box::new(Expr::Assign(AssignExpr {
        span: DUMMY_SP,
        op: AssignOp::Assign,
        left: AssignTarget::Simple(SimpleAssignTarget::Member(MemberExpr {
            span: DUMMY_SP,
            obj,
            prop: MemberProp::Ident(IdentName::new(prop.into(), DUMMY_SP)),
        })),
        right: value,
    })))
}

/// `case "a": return e;` for each pair, in one `switch`, as the body of a function over a closed union.
pub fn module_stmt(stmt: Stmt, span: Span) -> ModuleItem {
    ModuleItem::Stmt(at(stmt, span))
}
