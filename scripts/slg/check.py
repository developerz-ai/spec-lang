"""Parse every .slg file and every ```slg fence with the grammar in docs/idea/25-grammar.md.

Form only: the grammar carries no types or meaning. Usage: check.py <path>...
"""
import pathlib
import re
import sys

from lark import Lark
from lark.exceptions import LarkError

ROOT = pathlib.Path(__file__).resolve().parents[2]
GRAMMAR_DOC = ROOT / "docs/idea/25-grammar.md"
FENCE = re.compile(r"^```slg\n(.*?)^```", re.S | re.M)


def grammar() -> str:
    text = GRAMMAR_DOC.read_text()
    block = re.search(r"```text\n(// spec-lang surface grammar.*?)```", text, re.S)
    if not block:
        sys.exit(f"no grammar block in {GRAMMAR_DOC}")
    return block.group(1)


def sources(paths):
    for p in paths:
        for f in sorted(pathlib.Path(p).rglob("*")) if pathlib.Path(p).is_dir() else [pathlib.Path(p)]:
            if f.suffix == ".slg":
                yield str(f), f.read_text(), 1
            elif f.suffix == ".md":
                text = f.read_text()
                for m in FENCE.finditer(text):
                    yield str(f), m.group(1), text[: m.start()].count("\n") + 2


def main() -> int:
    parser = Lark(grammar(), start="file", parser="lalr", strict=True)
    failed = checked = 0
    for name, src, line in sources(sys.argv[1:]):
        checked += 1
        try:
            parser.parse(src)
        except LarkError as e:
            failed += 1
            print(f"{name}:{line}: does not parse\n{e}", file=sys.stderr)
    print(f"slg: {checked} blocks, {failed} failed")
    return 1 if failed or not checked else 0


if __name__ == "__main__":
    sys.exit(main())
