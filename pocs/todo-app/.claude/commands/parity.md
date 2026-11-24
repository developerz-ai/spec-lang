---
description: Run the whole parity suite and triage every failure to spec, contract or backend.
---

# /parity

1. `bin/build` · a backend fails to build → that backend's `bin/check`, report, continue with the rest.
2. `bun scripts/parity/run.ts`, then `--mix`, then `--race 40`.
3. For each failure decide who is wrong, in this order:
   - the case contradicts `src/` → the case is wrong; fix it and say so.
   - `src/` does not decide it and the contract does → the backend is wrong; `/project <backend>`.
   - neither decides it → an open choice: settle it in `docs/contract.md`, add the case, write it in `findings.md` under Spec errors.
4. A failure only under `--mix` is a store format or lock disagreement: diff the two backends' `store/` code against contract section 3.
5. Report: one line per failure with its verdict; nothing else.
