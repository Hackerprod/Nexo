"""J14 B1-R3 exact Pattern diagnostics and the EOF-only FIRST delta.

Usage: python -B tests/j14_r3_manifest.py OLD_DEBUG NEW_DEBUG [OLD_RELEASE NEW_RELEASE]
"""

import sys

from j14_r1_manifest import expected, run, span

P = ["(", "-", "IntLiteral", "Name", "_", "edit", "false", "move", "true"]
OLD_P = ["(", "IntLiteral", "Name", "_", "edit", "false", "move", "true"]
CASES = (
    ("fn Main(){let o:Option<I64>=Some(1);match o{Some(n,)=>{},None=>{}}}", ")", 51, "grammar.Pattern", P, 44, "symbol"),
    ("fn Main(){match 1{1=>{},-}}", "}", 25, "signed_integer", ["IntLiteral"], 24, "symbol"),
    ("fn Main(){match 1{-=>{},_=>{}}}", "=>", 19, "signed_integer", ["IntLiteral"], 18, "symbol"),
    ("fn Main(){match 1{?=>{},_=>{}}}", "?", 18, "grammar.Pattern", P, 18, "symbol"),
    ("fn Main(){let o:Option<I64>=Some(1);match o{Some(?)=>{},None=>{}}}", "?", 49, "grammar.Pattern", P, 49, "symbol"),
    ("fn Main(){match 1{1=>{},", "", 24, "grammar.Arm", P, 10, "eof"),
)


def wanted(token, start, context, first, prefix, kind):
    d = expected("NX-E0002", token, start, context, first, prefix)
    d["unexpected_token"]["kind"] = kind
    d["unexpected_token"]["span"] = span(start, start + len(token))
    return d


def main():
    pairs = list(zip(sys.argv[1::2], sys.argv[2::2]))
    assert len(pairs) in (1, 2)
    for source, token, start, context, first, prefix, kind in CASES:
        for old, new in pairs:
            for command in ("check", "run"):
                fresh = run(new, command, source)
                assert fresh["diagnostics"] == [wanted(token, start, context, first, prefix, kind)], (source, command, fresh)
                assert fresh["outcome"] == {"kind": "rejected"}
                assert fresh["stdout"] == fresh["events"] == fresh["observations"] == []
                if kind == "eof":
                    before = run(old, command, source)
                    assert before["diagnostics"] == [wanted(token, start, context, OLD_P, prefix, kind)]
                    assert before["diagnostics"][0]["parser_context"] == fresh["diagnostics"][0]["parser_context"]
                    assert before["diagnostics"][0]["recognized_prefix"] == fresh["diagnostics"][0]["recognized_prefix"]
                    assert before["diagnostics"][0]["unexpected_token"] == fresh["diagnostics"][0]["unexpected_token"]
                    changed = before["diagnostics"][0].copy()
                    changed["expected_tokens"] = P
                    assert changed == fresh["diagnostics"][0], (source, command, before, fresh)
    print(f"R3: {len(CASES)} exact Pattern diagnostics; EOF delta is only '-' in FIRST, {len(pairs)} binary pair(s)")


if __name__ == "__main__":
    main()
