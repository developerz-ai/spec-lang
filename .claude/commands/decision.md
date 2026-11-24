---
description: Record a one-way door as docs/decisions/NNN-<slug>.md and wire it into the index.
argument-hint: <the decision in one line>
---

# /decision

1. `ls docs/decisions` → next number. Read two existing records for tone: short, dated, no hedging.
2. Write `docs/decisions/NNN-<slug>.md`, nothing more than:
   - `# NNN Title`, `**Status:** proposed|accepted, <today>. <what is built, honestly>`
   - **Decision.** one paragraph · **Why.** the reason that would survive a sceptic · **Rejected.** each alternative and why
   - **Cost accepted.** · **Revisit when** a condition someone can observe · **Not claimed.** anything that sounds like a number
3. If it changes the language: change `docs/idea/01-language-card.md` first, then every doc that uses the changed part, in the same commit.
4. Add the line to `docs/decisions/README.md`.
5. `bin/check docs` · red → fix.

Decision: $ARGUMENTS
