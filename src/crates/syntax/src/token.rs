//! Tokens. Words are not split into keywords here: whether `else` or `in` is a
//! keyword depends on where the parser stands, as in the Lark contextual lexer.

use crate::span::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// `NAME` text; a keyword when the parser can take that keyword here.
    Word,
    /// `NUM`, with its unit suffix.
    Num,
    /// `TEXT`, with its optional lower-case prefix and both quotes.
    Text,
    Punct(Punct),
    /// Text no terminal matches. Kept as a token, not raised, because the
    /// parser may split a word and re-lex what follows, which changes what
    /// the rest of the line is; the error is raised if the parser reaches it.
    Error(LexError),
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LexError {
    UnexpectedChar,
    UnterminatedText,
}

/// Every symbol terminal of the grammar. Unicode spellings map to the same
/// variant as their ASCII form; [`Punct::Sym`] keeps which one was written
/// only where the AST needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Punct {
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Semi,
    Colon,
    Dot,
    Eq,
    FatArrow,
    /// `->` or `→`.
    Arrow,
    At,
    Question,
    Bar,
    Caret,
    Sup2,
    Sup3,
    Plus,
    Minus,
    /// `*` or `×`.
    Star,
    /// `/` or `÷`.
    Slash,
    PlusMinus,
    RangeIncl,
    RangeExcl,
    EqEq,
    /// `!=` or `≠`.
    NotEq,
    /// `<=` or `≤`.
    Le,
    /// `>=` or `≥`.
    Ge,
    Lt,
    Gt,
    /// `∈`, the symbol spelling of `in`.
    Elem,
    /// `∧`.
    AndSym,
    /// `∨`.
    OrSym,
    /// `¬`.
    NotSym,
    /// `⇒`.
    ImpliesSym,
}

impl Punct {
    /// The ASCII spelling the formatter prints.
    pub fn ascii(self) -> &'static str {
        use Punct::*;
        match self {
            LParen => "(",
            RParen => ")",
            LBracket => "[",
            RBracket => "]",
            LBrace => "{",
            RBrace => "}",
            Comma => ",",
            Semi => ";",
            Colon => ":",
            Dot => ".",
            Eq => "=",
            FatArrow => "=>",
            Arrow => "->",
            At => "@",
            Question => "?",
            Bar => "|",
            Caret => "^",
            Sup2 => "²",
            Sup3 => "³",
            Plus => "+",
            Minus => "-",
            Star => "*",
            Slash => "/",
            PlusMinus => "±",
            RangeIncl => "..=",
            RangeExcl => "..<",
            EqEq => "==",
            NotEq => "!=",
            Le => "<=",
            Ge => ">=",
            Lt => "<",
            Gt => ">",
            Elem => "in",
            AndSym => "and",
            OrSym => "or",
            NotSym => "not",
            ImpliesSym => "implies",
        }
    }
}

/// Symbol spellings, longest first so that a prefix never wins.
pub(crate) const PUNCTS: &[(&str, Punct)] = &[
    ("..=", Punct::RangeIncl),
    ("..<", Punct::RangeExcl),
    ("->", Punct::Arrow),
    ("=>", Punct::FatArrow),
    ("==", Punct::EqEq),
    ("!=", Punct::NotEq),
    ("<=", Punct::Le),
    (">=", Punct::Ge),
    ("(", Punct::LParen),
    (")", Punct::RParen),
    ("[", Punct::LBracket),
    ("]", Punct::RBracket),
    ("{", Punct::LBrace),
    ("}", Punct::RBrace),
    (",", Punct::Comma),
    (";", Punct::Semi),
    (":", Punct::Colon),
    (".", Punct::Dot),
    ("=", Punct::Eq),
    ("@", Punct::At),
    ("?", Punct::Question),
    ("|", Punct::Bar),
    ("^", Punct::Caret),
    ("²", Punct::Sup2),
    ("³", Punct::Sup3),
    ("+", Punct::Plus),
    ("-", Punct::Minus),
    ("*", Punct::Star),
    ("×", Punct::Star),
    ("/", Punct::Slash),
    ("÷", Punct::Slash),
    ("±", Punct::PlusMinus),
    ("<", Punct::Lt),
    (">", Punct::Gt),
    ("≠", Punct::NotEq),
    ("≤", Punct::Le),
    ("≥", Punct::Ge),
    ("→", Punct::Arrow),
    ("∈", Punct::Elem),
    ("∧", Punct::AndSym),
    ("∨", Punct::OrSym),
    ("¬", Punct::NotSym),
    ("⇒", Punct::ImpliesSym),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
