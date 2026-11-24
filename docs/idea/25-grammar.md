# Grammar

> **Design.** Nothing here runs yet except this grammar: in Lark strict mode it builds as LALR(1) with no conflicts, and every `slg` block in the docs parsed with it on 2026-11-24.

It carries no types or meaning. Unicode operators are one-to-one spellings of the ASCII form, which the formatter prints.

```text
// spec-lang surface grammar, design draft. LALR(1). Parses; carries no types or meaning.
// Unicode operators are 1:1 spellings of the ASCII form; the formatter prints the ASCII form.
file: use* decl*
use: "use" path ";"
decl: note* "pub"? path NAME generics? params? result? tail
tail: ";" | children | "=" body
note: "@" path ("(" args? ")")?
generics: "[" param ("," param)* ","? "]"
params: "(" (param ("," param)* ","?)? ")"
param: note* NAME (":" expr)? ("=" expr)?
result: ":" wexpr | ARROW wexpr
children: "{" decl* "}"
body: value | "machine" "{" stmt* "}" | "foreign" NAME TEXT ";"
value: expr ";" | blocky
blocky: "{" stmt* "}" | match
match: "match" expr "{" arm* "}"
arm: bar "=>" value
stmt: ("let"|"var") NAME (":" wexpr)? "=" value
    | "return" value | "return" ";" | "assert" expr ";"
    | expr ";" | expr "=" value
    | "for" NAME IN expr "{" stmt* "}"
    | "while" expr "{" stmt* "}"
    | ifs | match
ifs: "if" expr "{" stmt* "}" ("else" (ifs | "{" stmt* "}"))?
?expr: edge | edge "=>" expr | edge "=>" "{" stmt* "}"
?edge: alt | edge ARROW alt
?alt: wexpr | alt "else" last | cond | fail
?last: wexpr | fail | cond
cond: "if" wexpr "then" wexpr "else" last
fail: "fail" wexpr
?wexpr: imp | wexpr "where" imp
?imp: or_ | or_ IMPLIES imp
?or_: and_ | or_ OR and_
?and_: not_ | and_ AND not_
?not_: cmp | NOT not_
?cmp: bar ((CMP|IN) bar)*
?bar: bits | bar "|" bits
?bits: range | range ("band"|"bor"|"bxor"|"shl"|"shr") range
?range: sum | sum (RNG|PM) sum
?sum: prod | sum SIGN prod
?prod: un | prod (MULOP|"mod") un
?un: pow | SIGN un | ("own"|"ref"|"mut"|"try") un
?pow: post | post "^" un | post SUP
?post: atom | post "." NAME | post "(" args? ")" | post "[" args "]"
args: arg ("," arg)* ","?
arg: (NAME ":")? expr
?atom: NAME | NUM | TEXT | "?" NAME | "(" args? ")" | "[" args? "]"
path: NAME ("." NAME)*
CMP: "==" | "!=" | "<=" | ">=" | "<" | ">" | "≠" | "≤" | "≥"
MULOP: "*" | "/" | "×" | "÷"
SIGN: "+" | "-"
PM: "±"
SUP: "²" | "³"
RNG: "..=" | "..<"
NUM.2: /0x[0-9a-fA-F_]+|[0-9][0-9_]*(\.[0-9]+)?(e-?[0-9]+)?([°µμ]?[A-Za-z][A-Za-z0-9]*|%)?/
TEXT: /([a-z]+)?"([^"\\\n]|\\.)*"/
NAME: /[^\W\d²³](?:(?![²³])\w)*/
IN.3: /in\b|∈/
ARROW: "->" | "→"
AND.3: /and\b|∧/
OR.3: /or\b|∨/
NOT.3: /not\b|¬/
IMPLIES.3: /implies\b|⇒/
COMMENT: /\/\/[^\n]*/
%ignore COMMENT
%ignore /\s+/
```

## Limits

The parser checks form only: an invented note parses, and a broken `{name}` inside a text literal parses. Both need the checker, which does not exist.
