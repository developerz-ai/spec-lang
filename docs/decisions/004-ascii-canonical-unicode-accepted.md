# 004 ASCII is canonical, maths symbols are accepted

**Status:** accepted, 2026-11-24, on tokenizer counts only.

**Decision.** One-to-one Unicode operators parse anywhere their ASCII form does; the formatter prints ASCII. `² ³ ±` and `°C` are canonical because they measured no worse anywhere and better somewhere.

**Rejected.** *Unicode canonical*: on one program it used fewer characters and more tokens on seven of nine tokenizers. *ASCII only*: needlessly forbids notation models know well.

**Caveat.** None of the tokenizers is a current Claude one, and tokens are not accuracy. Revisit on an accuracy trial ([02](../idea/02-syntax.md)).
